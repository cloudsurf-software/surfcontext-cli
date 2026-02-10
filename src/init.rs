use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::path::Path;

/// Repo template types.
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum RepoType {
    Product,
    CommandCenter,
}

/// Scaffold a new ARDS repo at the given path.
pub fn init_repo(
    path: Option<&str>,
    repo_type: RepoType,
    minimal: bool,
    quiet: bool,
) -> Result<()> {
    let target = match path {
        Some(p) => std::path::PathBuf::from(p),
        None => std::env::current_dir()?,
    };

    let project_name = target
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "my-project".to_string());

    if !quiet {
        println!(
            "{} {} at {}",
            "Initializing".green().bold(),
            project_name,
            target.display()
        );
    }

    // Create directory structure
    fs::create_dir_all(&target)?;

    let context_dir = target.join(".context");
    fs::create_dir_all(&context_dir)?;

    // Write surfcontext.json
    let config = generate_surfcontext_json(&project_name, &repo_type, minimal);
    fs::write(target.join("surfcontext.json"), config)?;
    if !quiet {
        println!("  {} surfcontext.json", "Created".green());
    }

    // Write CONTEXT.md
    let context_md = generate_context_md(&project_name, &repo_type);
    fs::write(target.join("CONTEXT.md"), context_md)?;
    if !quiet {
        println!("  {} CONTEXT.md", "Created".green());
    }

    if !minimal {
        // Create canonical directories
        for dir in &["agents", "docs", "skills", "guides"] {
            let d = context_dir.join(dir);
            fs::create_dir_all(&d)?;
        }

        // Create .claude/ with symlinks
        let claude_dir = target.join(".claude");
        fs::create_dir_all(&claude_dir)?;

        #[cfg(unix)]
        for name in &["agents", "docs", "skills", "guides"] {
            let link = claude_dir.join(name);
            let target_path = Path::new("../.context").join(name);
            if !link.exists() {
                std::os::unix::fs::symlink(&target_path, &link)
                    .with_context(|| format!("Failed to create symlink for {name}"))?;
            }
        }

        // Write example agent
        let agent_content = generate_example_agent(&project_name);
        fs::write(context_dir.join("agents/example.md"), agent_content)?;

        // Write example doc
        let doc_content = generate_example_doc(&project_name);
        fs::write(context_dir.join("docs/overview.md"), doc_content)?;

        // Write queue
        fs::write(context_dir.join("queue.md"), "# Task Queue\n\nNo tasks.\n")?;

        if matches!(repo_type, RepoType::CommandCenter) {
            fs::create_dir_all(target.join("plans"))?;
            fs::create_dir_all(target.join("research"))?;
        }

        if !quiet {
            println!("  {} .context/agents/, docs/, skills/, guides/", "Created".green());
            println!("  {} .claude/ symlinks", "Created".green());
        }
    }

    // Generate CLAUDE.md from CONTEXT.md
    let context_content = fs::read_to_string(target.join("CONTEXT.md"))?;
    let claude_md = generate_claude_md_from_context(&context_content);
    fs::write(target.join("CLAUDE.md"), claude_md)?;
    if !quiet {
        println!("  {} CLAUDE.md", "Generated".green());
    }

    if !quiet {
        println!();
        println!("{}", "Done! Next steps:".bold());
        println!("  1. Edit CONTEXT.md with your project details");
        println!("  2. Run `surf sync` to regenerate platform files");
    }

    Ok(())
}

fn generate_surfcontext_json(name: &str, repo_type: &RepoType, minimal: bool) -> String {
    let platforms = match repo_type {
        RepoType::Product => r#"["claude"]"#,
        RepoType::CommandCenter => r#"["claude", "codex"]"#,
    };

    let plans_line = match repo_type {
        RepoType::CommandCenter => {
            r#"
    "checkpointsDir": "plans/sessions",
    "plansDir": "plans""#
        }
        RepoType::Product => "",
    };

    let generation = if minimal {
        r#"{
    "claude": {
      "rootContext": "CLAUDE.md",
      "method": "symlink",
      "rootContextMethod": "sed-copy"
    }
  }"#
    } else {
        match repo_type {
            RepoType::Product => {
                r#"{
    "claude": {
      "rootContext": "CLAUDE.md",
      "agentsDir": ".claude/agents",
      "docsDir": ".claude/docs",
      "method": "symlink",
      "rootContextMethod": "sed-copy"
    }
  }"#
            }
            RepoType::CommandCenter => {
                r#"{
    "claude": {
      "rootContext": "CLAUDE.md",
      "agentsDir": ".claude/agents",
      "docsDir": ".claude/docs",
      "method": "symlink",
      "rootContextMethod": "sed-copy"
    },
    "codex": {
      "rootContext": "AGENTS.md",
      "method": "template-copy"
    }
  }"#
            }
        }
    };

    format!(
        r#"{{
  "version": "3.0",
  "platforms": {platforms},
  "canonical": {{
    "rootContext": "CONTEXT.md",
    "agentsDir": ".context/agents",
    "docsDir": ".context/docs",
    "skillsDir": ".context/skills",
    "guidesDir": ".context/guides"{plans_line}
  }},
  "generation": {generation},
  "discoveryOrder": [
    "CONTEXT.md",
    "surfcontext.json",
    ".context/docs/",
    ".context/guides/",
    ".context/agents/",
    ".context/skills/",
    ".context/queue.md"
  ],
  "ipSafety": {{
    "enabled": true,
    "owner": "{name}",
    "noAiCoAuthor": true,
    "noSecrets": true
  }}
}}
"#
    )
}

fn generate_context_md(name: &str, repo_type: &RepoType) -> String {
    match repo_type {
        RepoType::Product => {
            format!(
                r#"# {name}

Brief description of the project.

## Key Files

| Path | Purpose |
|------|---------|
| `.context/docs/overview.md` | Project overview |

## Architecture

Describe your architecture here.

## Stack & Development

**Stack**: TODO
**How to work**: TODO

## Core Principles

1. TODO

## Key Repositories

This is a standalone repo.
"#
            )
        }
        RepoType::CommandCenter => {
            format!(
                r#"# {name} Command Center

Strategic command center for {name}.

## Key Files

| Path | Purpose |
|------|---------|
| `.context/docs/overview.md` | Project overview |
| `.context/queue.md` | Shared task queue |
| `plans/` | Strategic plans |

## Architecture

```
{name}/
  CONTEXT.md          <- Source of truth
  .context/agents/    <- Agent definitions
  .context/docs/      <- Deep context docs
  plans/              <- Strategic plans
  research/           <- Research and analysis
```

## Stack & Development

**Stack**: Markdown, Claude Code agents
**How to work**: Open in Claude Code. Plans go to `plans/`.

## Core Principles

1. TODO

## Key Repositories

This is the command center repo.
"#
            )
        }
    }
}

fn generate_example_agent(name: &str) -> String {
    format!(
        "# Example Agent\n\n\
         You are a helpful agent for {name}. Customize this file.\n"
    )
}

fn generate_example_doc(name: &str) -> String {
    format!(
        "# {name} Overview\n\n\
         Project overview goes here.\n"
    )
}

/// Simple CLAUDE.md generation (same as generate.rs but standalone for init).
fn generate_claude_md_from_context(content: &str) -> String {
    let mut output = String::with_capacity(content.len() + 200);
    output.push_str("<!-- SurfContext ARDS v3.0 — surfcontext.org -->\n");
    output.push_str("<!-- GENERATED — edit CONTEXT.md, then run scripts/surfcontext-sync.sh -->\n");

    let transformed = content
        .replace(".context/docs/", ".claude/docs/")
        .replace(".context/agents/", ".claude/agents/")
        .replace(".context/skills/", ".claude/skills/")
        .replace(".context/guides/", ".claude/guides/")
        .replace(".context/queue.md", ".claude/queue.md")
        .replace(".context/", ".claude/");

    output.push_str(&transformed);
    output
}
