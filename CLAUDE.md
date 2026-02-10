<!-- SurfContext ARDS v3.0 — surfcontext.org -->
<!-- GENERATED — edit CONTEXT.md, then run scripts/surfcontext-sync.sh -->
# SurfContext CLI

Rust CLI for the SurfContext/ARDS v3.0 standard. Replaces surfcontext-sync.sh (275 lines bash) and surfcontext-sync.js (580 lines Node.js) with a single portable binary.

## Key Files

| Path | Purpose |
|------|---------|
| `src/main.rs` | Clap entry point — `surf sync` and `surf init` |
| `src/config.rs` | surfcontext.json serde model |
| `src/sync/mod.rs` | Sync orchestrator |
| `src/sync/local.rs` | Structure setup, symlinks, defensive sweep |
| `src/sync/generate.rs` | CLAUDE.md / AGENTS.md generation |
| `src/sync/cross_repo.rs` | Cross-repo file sync with SHA-256 |
| `src/init.rs` | Scaffold new ARDS repos |

## Architecture

Single Rust crate (not a workspace). Binary name: `surf`. Pure sync I/O — no async runtime.

```
surfcontext-cli/
  Cargo.toml
  src/
    main.rs                     # Clap CLI
    config.rs                   # surfcontext.json model
    sync/
      mod.rs                    # Orchestrator
      local.rs                  # Local ops (structure, symlinks, queue, defensive)
      generate.rs               # Platform file generation
      cross_repo.rs             # Cross-repo SHA-256 sync
    init.rs                     # Repo scaffolding
```

## Stack & Development

**Stack**: Rust 1.93+, clap 4, serde, sha2, walkdir, colored
**How to work**: `cargo build`, `cargo test`, `cargo clippy -- -D warnings`

## Commands

```
surf sync                       # Full sync pipeline
surf sync --dry-run             # Show what would change
surf sync --verbose             # Detailed output
surf sync --force               # Overwrite even if target newer
surf sync --local-only          # Skip cross-repo sync
surf init [path]                # Scaffold new ARDS repo
surf init --type product        # Product repo template
surf init --type command-center # Command center template
surf init --minimal             # Minimal ARDS setup
```

## Core Principles

1. Single binary, zero runtime dependencies
2. Identical behavior to the bash + JS scripts it replaces
3. Cross-platform: symlinks on unix, copies on windows
