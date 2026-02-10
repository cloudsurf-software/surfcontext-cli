use anyhow::{Context, Result};
use colored::Colorize;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use super::{SyncOpts, SyncReport};
use crate::config::SurfConfig;

/// Result status for a single file sync operation.
#[derive(Debug, PartialEq)]
enum FileStatus {
    New,
    Updated,
    Unchanged,
    SkippedNewer,
}

/// Sync all cross-repo targets defined in config.
pub fn sync_repos(
    repo_root: &Path,
    config: &SurfConfig,
    opts: &SyncOpts,
    report: &mut SyncReport,
) -> Result<()> {
    if config.sync.is_empty() {
        if !opts.quiet {
            println!("  {}", "No cross-repo sync targets configured.".dimmed());
        }
        return Ok(());
    }

    for (section_name, section) in &config.sync {
        let source_dir = repo_root.join(&section.source);
        let source_files = list_files_recursive(&source_dir)?;

        if !opts.quiet {
            println!(
                "\n  [{}] Source: {} ({} files)",
                section_name,
                section.source,
                source_files.len()
            );
        }

        for target in &section.targets {
            let target_repo_dir = repo_root.join(&target.repo);
            let target_dir = target_repo_dir.join(&target.dest);
            let label = format!("{}/{}", target.repo, target.dest);

            if !target_repo_dir.exists() {
                if !opts.quiet {
                    println!("  -> {}", label);
                    println!(
                        "     {} Repo not found: {} -- skipping",
                        "[WARN]".yellow(),
                        target_repo_dir.display()
                    );
                }
                continue;
            }

            if !opts.quiet {
                println!("  -> {}", label);
            }

            // Filter source files by include/exclude
            let filtered: Vec<&PathBuf> = source_files
                .iter()
                .filter(|f| matches_include(f, target.include.as_deref()))
                .filter(|f| !matches_exclude(f, target.exclude.as_deref()))
                .collect();

            let mut section_new = 0;
            let mut section_updated = 0;
            let mut section_unchanged = 0;
            let mut section_skipped = 0;

            for rel_path in &filtered {
                let src = source_dir.join(rel_path);
                let dst = target_dir.join(rel_path);

                let status = sync_single_file(&src, &dst, opts)?;

                match status {
                    FileStatus::New => {
                        section_new += 1;
                        if !opts.quiet {
                            println!(
                                "     {} {} (new)",
                                "+".green(),
                                rel_path.display()
                            );
                        }
                    }
                    FileStatus::Updated => {
                        section_updated += 1;
                        if !opts.quiet {
                            println!(
                                "     {} {} (updated)",
                                "~".yellow(),
                                rel_path.display()
                            );
                        }
                    }
                    FileStatus::Unchanged => {
                        section_unchanged += 1;
                        if opts.verbose && !opts.quiet {
                            println!(
                                "     {} {} (unchanged)",
                                "-".dimmed(),
                                rel_path.display()
                            );
                        }
                    }
                    FileStatus::SkippedNewer => {
                        section_skipped += 1;
                        if !opts.quiet {
                            println!(
                                "     {} {} (skipped — target newer)",
                                "!".yellow(),
                                rel_path.display()
                            );
                        }
                    }
                }
            }

            // Print summary for this target if not verbose
            if !opts.verbose && !opts.quiet && section_unchanged > 0 {
                println!(
                    "     {} {} file(s) unchanged",
                    "-".dimmed(),
                    section_unchanged
                );
            }

            report.cross_repo_new += section_new;
            report.cross_repo_updated += section_updated;
            report.cross_repo_unchanged += section_unchanged;
            report.cross_repo_skipped += section_skipped;
        }
    }

    Ok(())
}

/// Sync a single file: compare hashes, check modification time, copy if needed.
fn sync_single_file(src: &Path, dst: &Path, opts: &SyncOpts) -> Result<FileStatus> {
    let src_hash = file_hash(src)?;

    if !dst.exists() {
        // New file
        if !opts.dry_run {
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(src, dst).with_context(|| {
                format!("Failed to copy {} -> {}", src.display(), dst.display())
            })?;
        }
        return Ok(FileStatus::New);
    }

    let dst_hash = file_hash(dst)?;

    if src_hash == dst_hash {
        return Ok(FileStatus::Unchanged);
    }

    // Content differs — check modification times unless --force
    if !opts.force {
        let src_mod = fs::metadata(src)?.modified()?;
        let dst_mod = fs::metadata(dst)?.modified()?;
        if dst_mod > src_mod {
            return Ok(FileStatus::SkippedNewer);
        }
    }

    if !opts.dry_run {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src, dst)?;
    }

    Ok(FileStatus::Updated)
}

/// Compute SHA-256 hash of file contents.
fn file_hash(path: &Path) -> Result<String> {
    let content = fs::read(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    Ok(format!("{:x}", hasher.finalize()))
}

/// Recursively list all files under a directory, returning relative paths.
fn list_files_recursive(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    if !dir.exists() {
        return Ok(files);
    }

    for entry in WalkDir::new(dir).min_depth(1).sort_by_file_name() {
        let entry = entry?;
        if entry.file_type().is_file() {
            let rel = entry
                .path()
                .strip_prefix(dir)
                .unwrap_or(entry.path())
                .to_path_buf();
            files.push(rel);
        }
    }

    Ok(files)
}

/// Check if a relative path matches the include filter.
/// Matches against: full relative path, filename, or top-level directory name.
/// If include is None, everything matches.
fn matches_include(rel_path: &Path, include: Option<&[String]>) -> bool {
    let Some(filters) = include else {
        return true;
    };

    let path_str = rel_path.to_string_lossy();
    let filename = rel_path
        .file_name()
        .map(|f| f.to_string_lossy())
        .unwrap_or_default();
    let top_dir = rel_path
        .components()
        .next()
        .map(|c| c.as_os_str().to_string_lossy())
        .unwrap_or_default();

    filters.iter().any(|f| {
        f == path_str.as_ref() || f == filename.as_ref() || f == top_dir.as_ref()
    })
}

/// Check if a relative path matches the exclude filter.
/// Same matching logic as include. If exclude is None, nothing is excluded.
fn matches_exclude(rel_path: &Path, exclude: Option<&[String]>) -> bool {
    let Some(filters) = exclude else {
        return false;
    };

    let path_str = rel_path.to_string_lossy();
    let filename = rel_path
        .file_name()
        .map(|f| f.to_string_lossy())
        .unwrap_or_default();
    let top_dir = rel_path
        .components()
        .next()
        .map(|c| c.as_os_str().to_string_lossy())
        .unwrap_or_default();

    filters.iter().any(|f| {
        f == path_str.as_ref() || f == filename.as_ref() || f == top_dir.as_ref()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_include_none() {
        let path = PathBuf::from("check-deployment/SKILL.md");
        assert!(matches_include(&path, None));
    }

    #[test]
    fn test_matches_include_by_top_dir() {
        let path = PathBuf::from("check-deployment/SKILL.md");
        let include = vec!["check-deployment".to_string()];
        assert!(matches_include(&path, Some(&include)));
    }

    #[test]
    fn test_matches_include_by_filename() {
        let path = PathBuf::from("some-dir/repo-registry.md");
        let include = vec!["repo-registry.md".to_string()];
        assert!(matches_include(&path, Some(&include)));
    }

    #[test]
    fn test_matches_include_no_match() {
        let path = PathBuf::from("other-skill/SKILL.md");
        let include = vec!["check-deployment".to_string()];
        assert!(!matches_include(&path, Some(&include)));
    }

    #[test]
    fn test_matches_exclude_none() {
        let path = PathBuf::from("anything.md");
        assert!(!matches_exclude(&path, None));
    }

    #[test]
    fn test_matches_exclude_by_name() {
        let path = PathBuf::from("secret.md");
        let exclude = vec!["secret.md".to_string()];
        assert!(matches_exclude(&path, Some(&exclude)));
    }

    #[test]
    fn test_file_hash_consistency() {
        // Write a temp file, hash it twice, ensure same result
        let dir = std::env::temp_dir().join("surfcontext-test-hash");
        let _ = fs::create_dir_all(&dir);
        let file = dir.join("test.txt");
        fs::write(&file, "hello world").unwrap();

        let h1 = file_hash(&file).unwrap();
        let h2 = file_hash(&file).unwrap();
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64); // SHA-256 hex length

        let _ = fs::remove_dir_all(&dir);
    }
}
