//! `surf build` — compile a .surf file into a static site with discovery metadata.
//!
//! Supports two modes:
//! - **Single-page**: `.surf` files without `::site`/`::page` blocks produce one `index.html`.
//! - **Multi-page**: `.surf` files with `::site` + `::page` blocks produce a directory of
//!   HTML files at their declared routes, with shared navigation.

use anyhow::Result;
use colored::Colorize;
use notify::{EventKind, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use surf_parse::{extract_site, render_site_page, PageConfig};

pub fn handle_build(file: &str, out_dir: &str, title: Option<&str>, quiet: bool) -> Result<()> {
    let file_path = Path::new(file);
    let content = std::fs::read_to_string(file_path)
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

    // Determine the source filename for discovery links
    let source_filename = file_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "source.surf".to_string());

    let out_path = Path::new(out_dir);

    // Extract site structure
    let (site_config, pages, loose_blocks) = extract_site(&result.doc);

    if site_config.is_some() && !pages.is_empty() {
        // Multi-page site build
        let site = site_config.unwrap();

        // Build nav items — surf-parse handles label capitalization via display_title()
        let nav_items: Vec<(String, String)> = pages
            .iter()
            .map(|p| (p.route.clone(), p.display_title()))
            .collect();

        // Create output directory
        std::fs::create_dir_all(out_path)
            .map_err(|e| anyhow::anyhow!("Failed to create '{}': {}", out_dir, e))?;

        let mut built_count = 0;

        for page in &pages {
            let config = PageConfig {
                source_path: source_filename.clone(),
                title: title.map(|t| t.to_string()),
                ..Default::default()
            };

            // If this is the "/" page and there are loose blocks, prepend them
            let effective_page = if page.route == "/" && !loose_blocks.is_empty() {
                let mut combined_children = loose_blocks.clone();
                combined_children.extend(page.children.clone());
                surf_parse::PageEntry {
                    route: page.route.clone(),
                    layout: page.layout.clone(),
                    title: page.title.clone(),
                    sidebar: page.sidebar,
                    children: combined_children,
                }
            } else {
                page.clone()
            };

            let html = render_site_page(&effective_page, &site, &nav_items, &config);

            // Route "/" → dist/index.html
            // Route "/about" → dist/about/index.html
            let page_dir = if page.route == "/" {
                out_path.to_path_buf()
            } else {
                let route_clean = page.route.trim_start_matches('/');
                out_path.join(route_clean)
            };

            std::fs::create_dir_all(&page_dir)
                .map_err(|e| anyhow::anyhow!("Failed to create '{}': {}", page_dir.display(), e))?;

            let index_path = page_dir.join("index.html");
            std::fs::write(&index_path, &html)
                .map_err(|e| anyhow::anyhow!("Failed to write '{}': {}", index_path.display(), e))?;

            built_count += 1;

            if !quiet {
                println!("  {} {} → {}", "page".dimmed(), page.route, index_path.display());
            }
        }

        // If there are loose blocks but no "/" page, create an index from them
        if !pages.iter().any(|p| p.route == "/") && !loose_blocks.is_empty() {
            let index_page = surf_parse::PageEntry {
                route: "/".to_string(),
                layout: None,
                title: site.name.clone(),
                sidebar: false,
                children: loose_blocks,
            };

            let config = PageConfig {
                source_path: source_filename.clone(),
                title: title.map(|t| t.to_string()),
                ..Default::default()
            };

            let html = render_site_page(&index_page, &site, &nav_items, &config);
            let index_path = out_path.join("index.html");
            std::fs::write(&index_path, &html)
                .map_err(|e| anyhow::anyhow!("Failed to write '{}': {}", index_path.display(), e))?;

            built_count += 1;

            if !quiet {
                println!("  {} / → {}", "page".dimmed(), index_path.display());
            }
        }

        // Copy source .surf file alongside the built output
        let source_dest = out_path.join(&source_filename);
        std::fs::copy(file_path, &source_dest)
            .map_err(|e| anyhow::anyhow!("Failed to copy source to '{}': {}", source_dest.display(), e))?;

        if !quiet {
            println!(
                "{} {} ({} pages → {})",
                "Built".green().bold(),
                source_filename,
                built_count,
                out_path.display(),
            );
        }
    } else {
        // Single-page fallback (current behavior)
        let config = PageConfig {
            source_path: source_filename.clone(),
            title: title.map(|t| t.to_string()),
            ..Default::default()
        };

        let html = result.doc.to_html_page(&config);

        // Create output directory
        std::fs::create_dir_all(out_path)
            .map_err(|e| anyhow::anyhow!("Failed to create '{}': {}", out_dir, e))?;

        // Write index.html
        let index_path = out_path.join("index.html");
        std::fs::write(&index_path, &html)
            .map_err(|e| anyhow::anyhow!("Failed to write '{}': {}", index_path.display(), e))?;

        // Copy source .surf file alongside the built output (discovery mechanism)
        let source_dest = out_path.join(&source_filename);
        std::fs::copy(file_path, &source_dest)
            .map_err(|e| anyhow::anyhow!("Failed to copy source to '{}': {}", source_dest.display(), e))?;

        if !quiet {
            println!("{} {}", "Built".green().bold(), index_path.display());
            println!("  {} {}", "source:".dimmed(), source_dest.display());
            println!(
                "  {} {}",
                "discovery:".dimmed(),
                format!(
                    "<link rel=\"alternate\" type=\"text/surfdoc\" href=\"{}\">",
                    source_filename
                )
            );
        }
    }

    Ok(())
}


/// Watch the source file for changes and rebuild on each save.
///
/// Debounces rapid events (e.g. editors that write in stages) with a 200ms window.
/// Ctrl+C exits cleanly.
pub fn watch_and_rebuild(file: &str, out_dir: &str, title: Option<&str>, quiet: bool) -> Result<()> {
    let file_path = std::fs::canonicalize(file)
        .map_err(|e| anyhow::anyhow!("Cannot resolve path '{}': {}", file, e))?;

    let watch_dir = file_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine parent directory of '{}'", file))?;

    println!(
        "{} {} for changes (Ctrl+C to stop)",
        "Watching".cyan().bold(),
        file
    );

    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(event) = res {
            let _ = tx.send(event);
        }
    })?;

    watcher.watch(watch_dir, RecursiveMode::NonRecursive)?;

    let mut last_rebuild = Instant::now();
    let debounce = Duration::from_millis(200);

    loop {
        match rx.recv_timeout(Duration::from_secs(1)) {
            Ok(event) => {
                // Only rebuild on modify/create events that affect our file
                let dominated = matches!(
                    event.kind,
                    EventKind::Modify(_) | EventKind::Create(_)
                );
                let affects_our_file = event.paths.iter().any(|p| {
                    p.canonicalize().ok().as_ref() == Some(&file_path)
                });

                if dominated && affects_our_file && last_rebuild.elapsed() > debounce {
                    // Small delay to let the editor finish writing
                    std::thread::sleep(Duration::from_millis(50));

                    match handle_build(file, out_dir, title, quiet) {
                        Ok(()) => {
                            last_rebuild = Instant::now();
                        }
                        Err(e) => {
                            eprintln!("{} {}", "Build error:".red().bold(), e);
                        }
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Keep looping
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                break;
            }
        }
    }

    Ok(())
}
