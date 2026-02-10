pub mod cross_repo;
pub mod generate;
pub mod local;

use anyhow::{Context, Result};
use colored::Colorize;
use std::path::{Path, PathBuf};

use crate::config;

/// Options passed from CLI to sync pipeline.
pub struct SyncOpts {
    pub dry_run: bool,
    pub verbose: bool,
    pub force: bool,
    pub local_only: bool,
    pub quiet: bool,
}

/// Aggregate report from the sync pipeline.
#[derive(Default)]
pub struct SyncReport {
    pub created: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub redirected: usize,
    pub warnings: Vec<String>,
    pub cross_repo_new: usize,
    pub cross_repo_updated: usize,
    pub cross_repo_unchanged: usize,
    pub cross_repo_skipped: usize,
}

impl SyncReport {
    pub fn print_summary(&self) {
        println!();
        println!("{}", "========================================".dimmed());
        println!("{}", "Sync complete!".green().bold());
        println!();

        let mut parts = Vec::new();
        if self.created > 0 {
            parts.push(format!("{} created", self.created));
        }
        if self.updated > 0 {
            parts.push(format!("{} updated", self.updated));
        }
        if self.redirected > 0 {
            parts.push(format!("{} redirected", self.redirected));
        }
        if self.unchanged > 0 {
            parts.push(format!("{} unchanged", self.unchanged));
        }

        if !parts.is_empty() {
            println!("Local: {}", parts.join(", "));
        }

        let mut cross_parts = Vec::new();
        if self.cross_repo_new > 0 {
            cross_parts.push(format!("{} new", self.cross_repo_new));
        }
        if self.cross_repo_updated > 0 {
            cross_parts.push(format!("{} updated", self.cross_repo_updated));
        }
        if self.cross_repo_unchanged > 0 {
            cross_parts.push(format!("{} unchanged", self.cross_repo_unchanged));
        }
        if self.cross_repo_skipped > 0 {
            cross_parts.push(format!("{} skipped (target newer)", self.cross_repo_skipped));
        }

        if !cross_parts.is_empty() {
            println!("Cross-repo: {}", cross_parts.join(", "));
        }

        for w in &self.warnings {
            println!("{} {}", "WARNING:".yellow(), w);
        }
    }
}

/// Find the repo root by walking up from CWD looking for CONTEXT.md or surfcontext.json.
fn find_repo_root() -> Result<PathBuf> {
    let cwd = std::env::current_dir().context("Failed to get current directory")?;
    let mut dir = cwd.as_path();

    loop {
        if dir.join("CONTEXT.md").exists() || dir.join("surfcontext.json").exists() {
            return Ok(dir.to_path_buf());
        }
        match dir.parent() {
            Some(parent) => dir = parent,
            None => {
                anyhow::bail!(
                    "No CONTEXT.md or surfcontext.json found in {} or any parent directory",
                    cwd.display()
                );
            }
        }
    }
}

/// Run the full sync pipeline.
pub fn run_sync(opts: &SyncOpts) -> Result<SyncReport> {
    let repo_root = find_repo_root()?;
    let config = config::load_config(&repo_root)?;

    if !opts.quiet {
        println!(
            "{} {} {}",
            "SurfContext Sync".bold(),
            format!("v{}", config.version).dimmed(),
            format!("— {}", repo_root.display()).dimmed()
        );
        println!("{}", "================================".dimmed());
        if opts.dry_run {
            println!("{}", "[DRY RUN] No files will be written.".yellow());
        }
    }

    let mut report = SyncReport::default();

    // 1. Ensure .context/ structure
    if !opts.quiet {
        println!();
        println!("{}", "[Setup] Verifying .context/ structure...".bold());
    }
    local::ensure_structure(&repo_root, &config, opts, &mut report)?;

    // 2. Setup symlinks/copies from .context/ -> .claude/
    if !opts.quiet {
        println!();
        println!("{}", "[Symlinks] Setting up .claude/ -> .context/ links...".bold());
    }
    local::setup_links(&repo_root, &config, opts, &mut report)?;

    // 3. Generate platform files (CLAUDE.md, AGENTS.md)
    if !opts.quiet {
        println!();
        println!("{}", "[Generate] Building platform files...".bold());
    }
    generate::generate_all(&repo_root, &config, opts, &mut report)?;

    // 4. Sync queue
    local::sync_queue(&repo_root, opts, &mut report)?;

    // 5. Defensive sweep
    if !opts.quiet {
        println!();
        println!("{}", "[Defensive] Scanning .claude/ for orphans...".bold());
    }
    local::defensive_sweep(&repo_root, opts, &mut report)?;

    // 6. Path reference audit — catch .claude/ references in .context/ source files
    if !opts.quiet {
        println!();
        println!("{}", "[Audit] Checking .context/ files for .claude/ path references...".bold());
    }
    local::audit_path_references(&repo_root, opts, &mut report)?;

    // 7. Cross-repo sync
    if !opts.local_only {
        if !opts.quiet {
            println!();
            println!("{}", "[Cross-Repo] Running cross-repo sync...".bold());
        }
        cross_repo::sync_repos(&repo_root, &config, opts, &mut report)?;
    }

    Ok(report)
}

/// Canonicalize a directory list from config for structure setup.
pub fn canonical_dirs(config: &config::SurfConfig) -> Vec<&str> {
    vec![
        &config.canonical.agents_dir,
        &config.canonical.docs_dir,
        &config.canonical.skills_dir,
        &config.canonical.guides_dir,
    ]
}

/// Get the list of directories that should be symlinked from .claude/ -> .context/.
pub fn link_mappings(config: &config::SurfConfig) -> Vec<(&str, &str)> {
    // Returns (context_dir, claude_subdir_name)
    vec![
        (&config.canonical.agents_dir as &str, "agents"),
        (&config.canonical.docs_dir as &str, "docs"),
        (&config.canonical.skills_dir as &str, "skills"),
        (&config.canonical.guides_dir as &str, "guides"),
    ]
}

/// Check if a path is a symlink pointing to the expected target.
pub fn is_symlink_to(link_path: &Path, expected: &Path) -> bool {
    match std::fs::read_link(link_path) {
        Ok(target) => target == expected,
        Err(_) => false,
    }
}
