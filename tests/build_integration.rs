//! Integration tests for `surf build` multi-page site generation.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn surf_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_surf"))
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn temp_out(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("surf-build-test").join(name);
    // Clean up from previous runs
    let _ = fs::remove_dir_all(&dir);
    dir
}

#[test]
fn multi_page_produces_correct_file_tree() {
    let out = temp_out("multi-page-tree");
    let status = Command::new(surf_bin())
        .args(["build", fixture("site.surf").to_str().unwrap(), "--out", out.to_str().unwrap(), "--quiet"])
        .status()
        .expect("failed to run surf build");

    assert!(status.success(), "surf build should succeed");

    // Check expected files exist
    assert!(out.join("index.html").exists(), "index.html should exist");
    assert!(out.join("about/index.html").exists(), "about/index.html should exist");
    assert!(out.join("pricing/index.html").exists(), "pricing/index.html should exist");

    // Source file should be copied
    assert!(out.join("site.surf").exists(), "source .surf file should be copied");

    let _ = fs::remove_dir_all(&out);
}

#[test]
fn multi_page_nav_links_in_each_page() {
    let out = temp_out("multi-page-nav");
    let status = Command::new(surf_bin())
        .args(["build", fixture("site.surf").to_str().unwrap(), "--out", out.to_str().unwrap(), "--quiet"])
        .status()
        .expect("failed to run surf build");

    assert!(status.success());

    // Check nav appears in each page
    for page_path in &["index.html", "about/index.html", "pricing/index.html"] {
        let html = fs::read_to_string(out.join(page_path))
            .unwrap_or_else(|_| panic!("should read {}", page_path));

        assert!(html.contains("surfdoc-site-nav"), "{} should have site nav", page_path);
        assert!(html.contains("/index.html"), "{} should have home nav link", page_path);
        assert!(html.contains("/about/index.html"), "{} should have about nav link", page_path);
        assert!(html.contains("/pricing/index.html"), "{} should have pricing nav link", page_path);
    }

    let _ = fs::remove_dir_all(&out);
}

#[test]
fn multi_page_site_config_in_output() {
    let out = temp_out("multi-page-config");
    let status = Command::new(surf_bin())
        .args(["build", fixture("site.surf").to_str().unwrap(), "--out", out.to_str().unwrap(), "--quiet"])
        .status()
        .expect("failed to run surf build");

    assert!(status.success());

    let index_html = fs::read_to_string(out.join("index.html")).unwrap();

    // Site name should appear in nav
    assert!(index_html.contains("Test Site"), "site name should appear in nav");

    // Accent color from site config should be applied as CSS variable override
    assert!(index_html.contains("#6366f1"), "accent color should be in CSS overrides");

    // Title should include site name
    assert!(index_html.contains("<title>"), "should have title tag");

    // Footer should have site name
    assert!(index_html.contains("surfdoc-site-footer"), "should have footer");

    let _ = fs::remove_dir_all(&out);
}

#[test]
fn single_page_fallback_no_site_blocks() {
    let out = temp_out("single-page-fallback");
    let status = Command::new(surf_bin())
        .args(["build", fixture("single.surf").to_str().unwrap(), "--out", out.to_str().unwrap(), "--quiet"])
        .status()
        .expect("failed to run surf build");

    assert!(status.success());

    // Should produce a single index.html (no subdirectories for routes)
    assert!(out.join("index.html").exists(), "index.html should exist");
    assert!(!out.join("about").exists(), "should not create route subdirectories");

    // The HTML should NOT have site nav (it's a single-page build)
    let html = fs::read_to_string(out.join("index.html")).unwrap();
    assert!(!html.contains("surfdoc-site-nav"), "single-page should not have site nav");

    // But it should be a valid SurfDoc page
    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("SurfDoc v0.1"));

    let _ = fs::remove_dir_all(&out);
}

#[test]
fn multi_page_active_nav_link() {
    let out = temp_out("multi-page-active");
    let status = Command::new(surf_bin())
        .args(["build", fixture("site.surf").to_str().unwrap(), "--out", out.to_str().unwrap(), "--quiet"])
        .status()
        .expect("failed to run surf build");

    assert!(status.success());

    // The about page should mark the about link as active
    let about_html = fs::read_to_string(out.join("about/index.html")).unwrap();
    assert!(
        about_html.contains("class=\"active\">About Us</a>"),
        "about page should have active class on About Us link"
    );

    // The home page should mark the home link as active
    let index_html = fs::read_to_string(out.join("index.html")).unwrap();
    assert!(
        index_html.contains("class=\"active\">Home</a>"),
        "index page should have active class on Home link"
    );

    let _ = fs::remove_dir_all(&out);
}

#[test]
fn multi_page_page_content_rendered() {
    let out = temp_out("multi-page-content");
    let status = Command::new(surf_bin())
        .args(["build", fixture("site.surf").to_str().unwrap(), "--out", out.to_str().unwrap(), "--quiet"])
        .status()
        .expect("failed to run surf build");

    assert!(status.success());

    // About page should have its content rendered
    let about_html = fs::read_to_string(out.join("about/index.html")).unwrap();
    assert!(about_html.contains("Founded in 2026"), "about page should render its content");
    assert!(about_html.contains("great tools"), "about page should have markdown content");

    // Pricing page should have its content
    let pricing_html = fs::read_to_string(out.join("pricing/index.html")).unwrap();
    assert!(pricing_html.contains("Free tier"), "pricing page should have its content");

    // Home page should have welcome content
    let index_html = fs::read_to_string(out.join("index.html")).unwrap();
    assert!(index_html.contains("Welcome"), "home page should have welcome content");

    let _ = fs::remove_dir_all(&out);
}
