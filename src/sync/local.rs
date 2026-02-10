use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::path::Path;

use super::{SyncOpts, SyncReport, canonical_dirs, is_symlink_to, link_mappings};
use crate::config::SurfConfig;

/// Ensure all canonical .context/ directories exist.
pub fn ensure_structure(
    repo_root: &Path,
    config: &SurfConfig,
    opts: &SyncOpts,
    report: &mut SyncReport,
) -> Result<()> {
    let context_dir = repo_root.join(".context");
    if !context_dir.exists() {
        if !opts.dry_run {
            fs::create_dir_all(&context_dir)
                .with_context(|| format!("Failed to create {}", context_dir.display()))?;
        }
        if !opts.quiet {
            println!("  {} .context/", "Created".green());
        }
        report.created += 1;
    }

    for dir in canonical_dirs(config) {
        let full = repo_root.join(dir);
        if !full.exists() {
            if !opts.dry_run {
                fs::create_dir_all(&full)
                    .with_context(|| format!("Failed to create {}", full.display()))?;
            }
            if !opts.quiet {
                println!("  {} {}/", "Created".green(), dir);
            }
            report.created += 1;
        } else if opts.verbose && !opts.quiet {
            println!("  {} {}/", "Exists".dimmed(), dir);
        }
    }

    Ok(())
}

/// Setup symlinks (unix) or copies (windows) from .claude/ -> .context/.
pub fn setup_links(
    repo_root: &Path,
    config: &SurfConfig,
    opts: &SyncOpts,
    report: &mut SyncReport,
) -> Result<()> {
    let claude_dir = repo_root.join(".claude");
    if !claude_dir.exists() && !opts.dry_run {
        fs::create_dir_all(&claude_dir)?;
    }

    for (context_dir, name) in link_mappings(config) {
        let source_full = repo_root.join(context_dir);
        if !source_full.exists() {
            if opts.verbose && !opts.quiet {
                println!("  {} {context_dir} does not exist", "[skip]".dimmed());
            }
            continue;
        }

        let claude_path = claude_dir.join(name);
        // Relative symlink target: ../.context/<name>
        let symlink_target = std::path::Path::new("..").join(context_dir);

        setup_single_link(
            repo_root,
            &claude_path,
            &symlink_target,
            &format!(".claude/{name}"),
            context_dir,
            opts,
            report,
        )?;
    }

    Ok(())
}

/// Setup a single symlink or copy, handling existing states.
fn setup_single_link(
    _repo_root: &Path,
    claude_path: &Path,
    symlink_target: &Path,
    display_claude: &str,
    display_context: &str,
    opts: &SyncOpts,
    report: &mut SyncReport,
) -> Result<()> {
    if claude_path.is_symlink() {
        // Already a symlink — check if it points to the right place
        if is_symlink_to(claude_path, symlink_target) {
            if !opts.quiet {
                println!(
                    "  {} -> {} {}",
                    display_claude,
                    display_context,
                    "(already linked)".dimmed()
                );
            }
            report.unchanged += 1;
        } else {
            // Wrong target — fix it
            if !opts.dry_run {
                fs::remove_file(claude_path)?;
                create_symlink(symlink_target, claude_path)?;
            }
            if !opts.quiet {
                println!(
                    "  {} -> {} {}",
                    display_claude,
                    display_context,
                    "(fixed link)".yellow()
                );
            }
            report.updated += 1;
        }
    } else if claude_path.is_dir() {
        // Real directory — convert to symlink
        if !opts.dry_run {
            fs::remove_dir_all(claude_path)?;
            create_symlink(symlink_target, claude_path)?;
        }
        if !opts.quiet {
            println!(
                "  {} -> {} {}",
                display_claude,
                display_context,
                "(converted dir to link)".yellow()
            );
        }
        report.updated += 1;
    } else if claude_path.is_file() {
        let msg = format!("{display_claude} is a file, expected directory or symlink");
        report.warnings.push(msg.clone());
        if !opts.quiet {
            println!("  {} {}", "ERROR:".red(), msg);
        }
    } else {
        // Doesn't exist — create
        if !opts.dry_run {
            create_symlink(symlink_target, claude_path)?;
        }
        if !opts.quiet {
            println!(
                "  {} -> {} {}",
                display_claude,
                display_context,
                "(created)".green()
            );
        }
        report.created += 1;
    }

    Ok(())
}

/// Create a symlink (unix) or copy directory (windows).
#[cfg(unix)]
fn create_symlink(target: &Path, link: &Path) -> Result<()> {
    std::os::unix::fs::symlink(target, link)
        .with_context(|| format!("Failed to create symlink {} -> {}", link.display(), target.display()))
}

#[cfg(windows)]
fn create_symlink(target: &Path, link: &Path) -> Result<()> {
    // Windows: symlinks require admin/dev mode, so copy instead
    let resolved = link.parent().unwrap_or(Path::new(".")).join(target);
    copy_dir_recursive(&resolved, link)
}

#[cfg(windows)]
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in walkdir::WalkDir::new(src).min_depth(1) {
        let entry = entry?;
        let rel = entry.path().strip_prefix(src)?;
        let dest_path = dst.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&dest_path)?;
        } else {
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), &dest_path)?;
        }
    }
    Ok(())
}

/// Sync .context/queue.md to .claude/queue.md with a redirect header.
pub fn sync_queue(
    repo_root: &Path,
    opts: &SyncOpts,
    report: &mut SyncReport,
) -> Result<()> {
    let source = repo_root.join(".context/queue.md");
    let target = repo_root.join(".claude/queue.md");

    if !source.exists() {
        return Ok(());
    }

    if !opts.quiet {
        println!();
        println!("{}", "[Queue] Syncing .context/queue.md -> .claude/queue.md...".bold());
    }

    let source_content = fs::read_to_string(&source)?;
    let output = format!(
        "<!-- DO NOT EDIT — generated from .context/queue.md by surf sync -->\n\
         <!-- Source of truth: .context/queue.md -->\n\
         \n\
         {source_content}"
    );

    // Check if target already matches
    if target.exists() {
        let existing = fs::read_to_string(&target)?;
        if existing == output {
            if !opts.quiet {
                println!("  {}", "unchanged".dimmed());
            }
            report.unchanged += 1;
            return Ok(());
        }
    }

    if !opts.dry_run {
        fs::write(&target, output)?;
    }
    if !opts.quiet {
        println!("  {}", "Done".green());
    }
    report.updated += 1;

    Ok(())
}

/// Scan .claude/ for orphaned files/dirs and redirect them.
pub fn defensive_sweep(
    repo_root: &Path,
    opts: &SyncOpts,
    report: &mut SyncReport,
) -> Result<()> {
    let claude_dir = repo_root.join(".claude");
    if !claude_dir.exists() {
        return Ok(());
    }

    let entries = fs::read_dir(&claude_dir)?;
    let mut found_orphan = false;

    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        let path = entry.path();

        // Skip symlinks (managed by setup_links)
        if path.is_symlink() {
            continue;
        }

        // Skip Claude Code native files (settings*.json)
        if name_str.starts_with("settings") && name_str.ends_with(".json") {
            continue;
        }

        // Skip queue.md (managed copy)
        if name_str == "queue.md" {
            continue;
        }

        // Handle orphan files
        if path.is_file() {
            found_orphan = true;
            if name_str.ends_with(".md") {
                if !opts.quiet {
                    println!("  {} orphan file: .claude/{}", "Redirecting".yellow(), name_str);
                }
                if !opts.dry_run {
                    let redirect = REDIRECT_FILE_CONTENT;
                    fs::write(&path, redirect)?;
                }
                report.redirected += 1;
            } else {
                let msg = format!(
                    "Unexpected file in .claude/: {} (not managed by SurfContext)",
                    name_str
                );
                if !opts.quiet {
                    println!("  {} {}", "WARNING:".yellow(), msg);
                }
                report.warnings.push(msg);
            }
        }

        // Handle orphan directories
        if path.is_dir() {
            found_orphan = true;
            let context_counterpart = repo_root.join(".context").join(&*name_str);
            if context_counterpart.is_dir() {
                // Has a .context/ counterpart — convert to symlink
                let symlink_target = std::path::Path::new("..").join(".context").join(&*name_str);
                if !opts.quiet {
                    println!(
                        "  {} orphan dir: .claude/{} -> .context/{}",
                        "Converting".yellow(),
                        name_str,
                        name_str
                    );
                }
                if !opts.dry_run {
                    fs::remove_dir_all(&path)?;
                    create_symlink(&symlink_target, &path)?;
                }
                report.updated += 1;
            } else {
                // No counterpart — add redirect README
                let msg = format!(
                    "Unexpected directory in .claude/: {} (no .context/ counterpart)",
                    name_str
                );
                if !opts.quiet {
                    println!("  {} {}", "WARNING:".yellow(), msg);
                }
                if !opts.dry_run {
                    let readme_path = path.join("README.md");
                    let content = format!(
                        "<!-- DEPRECATED — this directory should not exist in .claude/ -->\n\n\
                         # Deprecated Directory\n\n\
                         This directory does not belong in `.claude/`. In SurfContext/ARDS v3.0, all content\n\
                         lives in `.context/` and is symlinked or synced to `.claude/` by `surf sync`.\n\n\
                         **To fix:** Move contents to `.context/{name_str}/` and run `surf sync`.\n"
                    );
                    fs::write(readme_path, content)?;
                }
                report.warnings.push(msg);
            }
        }
    }

    if !found_orphan && !opts.quiet {
        println!("  {}", "Clean".green());
    }

    Ok(())
}

const REDIRECT_FILE_CONTENT: &str = "\
<!-- DEPRECATED — this file has moved to .context/ -->
<!-- This file is auto-managed by surf sync -->

# This file has been deprecated

The contents of this file have been moved. Do not edit this file.

**Go to:** `.context/` — that is the source of truth for all guides, docs, agents, skills, and queue.

To find the canonical version, check:
- `.context/docs/` — deep context documents
- `.context/guides/` — living how-to guides
- `.context/agents/` — agent definitions
- `.context/skills/` — skill definitions
- `.context/queue.md` — shared task queue
";
