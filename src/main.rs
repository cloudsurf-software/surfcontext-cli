use anyhow::Result;
use clap::{Parser, Subcommand};

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
    }

    Ok(())
}
