use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;

mod config;
mod init;
mod sync;

#[derive(Parser)]
#[command(name = "surf", version, about = "CLI for the SurfContext/ARDS v3.0 standard")]
struct Cli {
    /// Suppress non-essential output
    #[arg(long, global = true)]
    quiet: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum RenderFormat {
    Terminal,
    Markdown,
    Html,
}

#[derive(Subcommand)]
enum Commands {
    /// Run full sync pipeline (replaces surfcontext-sync.sh + .js)
    Sync {
        /// Show what would change without writing
        #[arg(long)]
        dry_run: bool,

        /// Show detailed output including unchanged files
        #[arg(long)]
        verbose: bool,

        /// Overwrite even if target is newer
        #[arg(long)]
        force: bool,

        /// Skip cross-repo sync
        #[arg(long)]
        local_only: bool,
    },

    /// Scaffold a new ARDS repo
    Init {
        /// Directory to initialize (default: current directory)
        path: Option<String>,

        /// Repo template type
        #[arg(long, value_enum, default_value = "product")]
        r#type: init::RepoType,

        /// Minimal setup (CONTEXT.md + surfcontext.json only)
        #[arg(long)]
        minimal: bool,
    },

    /// Render a SurfDoc file
    Render {
        /// Path to the .surf or .md file
        file: String,

        /// Output format
        #[arg(long, value_enum, default_value = "terminal")]
        format: RenderFormat,
    },

    /// Validate SurfDoc file(s)
    Validate {
        /// Path to the .surf or .md file(s)
        files: Vec<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Sync {
            dry_run,
            verbose,
            force,
            local_only,
        } => {
            let opts = sync::SyncOpts {
                dry_run,
                verbose,
                force,
                local_only,
                quiet: cli.quiet,
            };
            let report = sync::run_sync(&opts)?;
            if !cli.quiet {
                report.print_summary();
            }
        }
        Commands::Init {
            path,
            r#type,
            minimal,
        } => {
            init::init_repo(path.as_deref(), r#type, minimal, cli.quiet)?;
        }
        Commands::Render { file, format } => {
            handle_render(&file, format)?;
        }
        Commands::Validate { files } => {
            handle_validate(&files)?;
        }
    }

    Ok(())
}

fn handle_render(file: &str, format: RenderFormat) -> Result<()> {
    let content = std::fs::read_to_string(file)
        .map_err(|e| anyhow::anyhow!("Failed to read '{}': {}", file, e))?;

    let result = surf_parse::parse(&content);

    // Print parse diagnostics to stderr
    for diag in &result.diagnostics {
        let line_info = match diag.span {
            Some(span) => format!("{}:{}", file, span.start_line),
            None => file.to_string(),
        };
        eprintln!("{}: {}", line_info, diag.message);
    }

    let output = match format {
        RenderFormat::Terminal => result.doc.to_terminal(),
        RenderFormat::Markdown => result.doc.to_markdown(),
        RenderFormat::Html => result.doc.to_html(),
    };

    println!("{output}");
    Ok(())
}

fn handle_validate(files: &[String]) -> Result<()> {
    let mut has_errors = false;

    for file in files {
        let content = std::fs::read_to_string(file)
            .map_err(|e| anyhow::anyhow!("Failed to read '{}': {}", file, e))?;

        let result = surf_parse::parse(&content);

        // Combine parse diagnostics with validation diagnostics
        let mut all_diagnostics = result.diagnostics;
        all_diagnostics.extend(result.doc.validate());

        if all_diagnostics.is_empty() {
            println!("{}: {}", file, "OK".green());
        } else {
            for diag in &all_diagnostics {
                let severity_str = match diag.severity {
                    surf_parse::Severity::Error => {
                        has_errors = true;
                        format!("{}", "error".red().bold())
                    }
                    surf_parse::Severity::Warning => {
                        format!("{}", "warning".yellow().bold())
                    }
                    surf_parse::Severity::Info => {
                        format!("{}", "info".cyan().bold())
                    }
                };

                let line_info = match diag.span {
                    Some(span) => format!("{}:{}", file, span.start_line),
                    None => file.to_string(),
                };

                let code_str = match &diag.code {
                    Some(c) => format!("[{}] ", c),
                    None => String::new(),
                };

                println!("{line_info}: {severity_str}: {code_str}{}", diag.message);
            }
        }
    }

    if has_errors {
        std::process::exit(1);
    }

    Ok(())
}
