use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::path::Path;

use super::{SyncOpts, SyncReport};
use crate::config::SurfConfig;

/// Generate all platform-specific files based on config.
pub fn generate_all(
    repo_root: &Path,
    config: &SurfConfig,
    opts: &SyncOpts,
    report: &mut SyncReport,
) -> Result<()> {
    let context_path = repo_root.join(&config.canonical.root_context);
    if !context_path.exists() {
        if !opts.quiet {
            println!(
                "  {} {} not found, skipping generation",
                "[skip]".dimmed(),
                config.canonical.root_context
            );
        }
        return Ok(());
    }

    let context_content = fs::read_to_string(&context_path)
        .with_context(|| format!("Failed to read {}", context_path.display()))?;

    for (platform, platform_gen) in &config.generation {
        let output_name = platform_gen.root_context.as_deref().unwrap_or(
            match platform.as_str() {
                "claude" => "CLAUDE.md",
                "codex" => "AGENTS.md",
                "cursor" => ".cursorrules",
                _ => "CLAUDE.md",
            },
        );

        let output_path = repo_root.join(output_name);
        let method = platform_gen
            .root_context_method
            .as_deref()
            .unwrap_or(&platform_gen.method);

        let generated = match method {
            "sed-copy" => generate_claude_md(&context_content),
            "template-copy" => generate_agents_md(&context_content),
            _ => generate_claude_md(&context_content),
        };

        // Check if output already matches
        let needs_write = if output_path.exists() {
            let existing = fs::read_to_string(&output_path)?;
            existing != generated
        } else {
            true
        };

        if needs_write {
            if !opts.dry_run {
                fs::write(&output_path, &generated)?;
            }
            let line_count = generated.lines().count();
            if !opts.quiet {
                println!(
                    "  {} {} ({} lines) {}",
                    "Generated".green(),
                    output_name,
                    line_count,
                    if opts.dry_run { "(dry run)" } else { "" }
                );
            }
            report.updated += 1;
        } else {
            if !opts.quiet {
                println!("  {} {}", output_name, "(unchanged)".dimmed());
            }
            report.unchanged += 1;
        }
    }

    Ok(())
}

/// Generate CLAUDE.md from CONTEXT.md by path-substituting .context/ -> .claude/.
fn generate_claude_md(context_content: &str) -> String {
    let mut output = String::with_capacity(context_content.len() + 500);

    output.push_str("<!-- SurfContext ARDS v3.0 — surfcontext.org -->\n");
    output.push_str("<!-- GENERATED — edit CONTEXT.md, then run surf sync -->\n");

    let transformed = context_content
        .replace(".context/docs/", ".claude/docs/")
        .replace(".context/agents/", ".claude/agents/")
        .replace(".context/skills/", ".claude/skills/")
        .replace(".context/guides/", ".claude/guides/")
        .replace(".context/queue.md", ".claude/queue.md")
        .replace(".context/", ".claude/");

    output.push_str(&transformed);
    output.push_str(PATH_ENFORCEMENT_RULE);
    output
}

/// Generate AGENTS.md from CONTEXT.md by stripping persona and some sections.
fn generate_agents_md(context_content: &str) -> String {
    let mut output = String::with_capacity(context_content.len() + 500);

    output.push_str("<!-- SurfContext ARDS v3.0 — surfcontext.org -->\n");
    output.push_str("<!-- GENERATED — edit CONTEXT.md, then run surf sync -->\n");
    output.push('\n');

    let mut state = AgentsState::Normal;
    let lines: Vec<&str> = context_content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        match state {
            AgentsState::Normal => {
                // Replace persona title
                if line.starts_with("# ") && line.contains("Advisor Agent") {
                    // Replace with neutral title, derive from the line
                    let repo_name = line
                        .trim_start_matches("# ")
                        .replace("Advisor Agent", "Repository")
                        .replace("Strategic ", "Strategy ");
                    output.push_str(&format!("# {repo_name}\n"));
                    i += 1;
                    continue;
                }

                // Replace persona paragraph ("You are the strategic advisor...")
                if line.starts_with("You are the strategic advisor for") {
                    output.push_str("This is the strategic command center for CloudSurf Software LLC, a bootstrapped software company. Contains business strategy, agent definitions, knowledge docs, patent drafts, academic research, and plans. Markdown and LaTeX only -- no application code.\n");
                    output.push('\n');
                    // Skip until next blank line
                    i += 1;
                    while i < lines.len() && !lines[i].is_empty() {
                        i += 1;
                    }
                    continue;
                }

                // Skip "## Your Expertise" through "## Core Principles"
                if line == "## Your Expertise" {
                    state = AgentsState::SkippingSection("## Core Principles");
                    i += 1;
                    continue;
                }

                // Skip "## Commands" through "## Key Repositories"
                if line == "## Commands" {
                    state = AgentsState::SkippingSection("## Key Repositories");
                    i += 1;
                    continue;
                }

                output.push_str(line);
                output.push('\n');
            }
            AgentsState::SkippingSection(target) => {
                if line == target {
                    // Found the target heading — emit it and return to normal
                    output.push_str(line);
                    output.push('\n');
                    state = AgentsState::Normal;
                }
                // Otherwise skip the line
            }
        }
        i += 1;
    }

    output.push_str(PATH_ENFORCEMENT_RULE);
    output
}

/// Rule injected into ALL generated platform files (CLAUDE.md, AGENTS.md, .cursorrules).
/// Tells agents to write to .context/ (source of truth), not .claude/ (symlinked).
const PATH_ENFORCEMENT_RULE: &str = "\n\n\
<!-- surf sync: path enforcement rule (injected automatically) -->\n\
## Source of Truth: `.context/`\n\
\n\
The `.claude/` directory is **generated** — its subdirectories (`docs/`, `agents/`, `guides/`, `skills/`) \
are symlinks to `.context/`. The queue file (`.claude/queue.md`) is a copy.\n\
\n\
**Rules for all agents (Claude, Codex, Cursor)**:\n\
- When **creating or editing** files in docs, agents, guides, or skills: always use `.context/` paths \
(e.g., `.context/docs/foo.md`, NOT `.claude/docs/foo.md`)\n\
- When **reading** files: either path works (symlinks resolve the same), but prefer `.context/`\n\
- When **telling the user** or **telling subagents** where a file is: always say `.context/`\n\
- When **referencing paths in documents** you write: always use `.context/`\n\
- **Never** create new directories inside `.claude/` — create them in `.context/` and run `surf sync`\n\
- To edit the queue: edit `.context/queue.md` (`.claude/queue.md` is overwritten by sync)\n\
\n\
This rule is enforced by `surf sync` which audits `.context/` files for accidental `.claude/` references.\n\
";

#[derive(Clone, Copy)]
enum AgentsState<'a> {
    Normal,
    SkippingSection(&'a str),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_md_path_transform() {
        let input = "See `.context/docs/foo.md` and `.context/agents/bar.md`.\n";
        let result = generate_claude_md(input);
        assert!(result.contains(".claude/docs/foo.md"));
        assert!(result.contains(".claude/agents/bar.md"));
        assert!(result.starts_with("<!-- SurfContext ARDS v3.0"));
    }

    #[test]
    fn test_agents_md_strips_persona() {
        let input = "# CloudSurf Strategic Advisor Agent\n\n\
            You are the strategic advisor for **CloudSurf Software LLC**, a bootstrapped company.\n\
            Provide expert guidance.\n\n\
            ## Key Files\n\nSome content.\n";

        let result = generate_agents_md(input);
        assert!(result.contains("# CloudSurf Strategy Repository"));
        assert!(!result.contains("You are the strategic advisor"));
        assert!(result.contains("This is the strategic command center"));
        assert!(result.contains("## Key Files"));
    }

    #[test]
    fn test_agents_md_strips_expertise() {
        let input = "## Some Section\n\nKeep this.\n\n\
            ## Your Expertise\n\n- Item 1\n- Item 2\n\n\
            ## Core Principles\n\nKeep this too.\n";

        let result = generate_agents_md(input);
        assert!(result.contains("## Some Section"));
        assert!(!result.contains("## Your Expertise"));
        assert!(!result.contains("Item 1"));
        assert!(result.contains("## Core Principles"));
        assert!(result.contains("Keep this too."));
    }

    #[test]
    fn test_agents_md_strips_commands() {
        let input = "## Working Style\n\nKeep this.\n\n\
            ## Commands\n\n- `/strategy` — foo\n- `/review` — bar\n\n\
            ## Key Repositories\n\nKeep repos.\n";

        let result = generate_agents_md(input);
        assert!(result.contains("## Working Style"));
        assert!(!result.contains("## Commands"));
        assert!(!result.contains("/strategy"));
        assert!(result.contains("## Key Repositories"));
        assert!(result.contains("Keep repos."));
    }
}
