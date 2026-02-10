use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Top-level surfcontext.json schema.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct SurfConfig {
    #[serde(default = "default_version")]
    pub version: String,

    #[serde(default)]
    pub platforms: Vec<String>,

    #[serde(default)]
    pub canonical: Canonical,

    #[serde(default)]
    pub generation: HashMap<String, PlatformGen>,

    #[serde(default)]
    pub sync: HashMap<String, SyncSection>,

    #[serde(default)]
    pub discovery_order: Vec<String>,

    #[serde(default)]
    pub ip_safety: Option<IpSafety>,
}

fn default_version() -> String {
    "3.0".to_string()
}

/// Canonical directory layout.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct Canonical {
    #[serde(default = "default_root_context")]
    pub root_context: String,

    #[serde(default = "default_agents_dir")]
    pub agents_dir: String,

    #[serde(default = "default_docs_dir")]
    pub docs_dir: String,

    #[serde(default = "default_skills_dir")]
    pub skills_dir: String,

    #[serde(default = "default_guides_dir")]
    pub guides_dir: String,

    #[serde(default)]
    pub checkpoints_dir: Option<String>,

    #[serde(default)]
    pub plans_dir: Option<String>,
}

impl Default for Canonical {
    fn default() -> Self {
        Self {
            root_context: default_root_context(),
            agents_dir: default_agents_dir(),
            docs_dir: default_docs_dir(),
            skills_dir: default_skills_dir(),
            guides_dir: default_guides_dir(),
            checkpoints_dir: None,
            plans_dir: None,
        }
    }
}

fn default_root_context() -> String {
    "CONTEXT.md".to_string()
}
fn default_agents_dir() -> String {
    ".context/agents".to_string()
}
fn default_docs_dir() -> String {
    ".context/docs".to_string()
}
fn default_skills_dir() -> String {
    ".context/skills".to_string()
}
fn default_guides_dir() -> String {
    ".context/guides".to_string()
}

/// Platform-specific generation config.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct PlatformGen {
    #[serde(default)]
    pub root_context: Option<String>,

    #[serde(default)]
    pub agents_dir: Option<String>,

    #[serde(default)]
    pub docs_dir: Option<String>,

    #[serde(default = "default_method")]
    pub method: String,

    #[serde(default)]
    pub root_context_method: Option<String>,

    #[serde(default)]
    pub root_context_script: Option<String>,
}

fn default_method() -> String {
    "symlink".to_string()
}

/// Cross-repo sync section (e.g. "skills", "docs").
#[derive(Debug, Deserialize)]
pub struct SyncSection {
    pub source: String,
    pub targets: Vec<SyncTarget>,
}

/// A single cross-repo sync target.
#[derive(Debug, Deserialize)]
pub struct SyncTarget {
    pub repo: String,
    pub dest: String,

    #[serde(default)]
    pub include: Option<Vec<String>>,

    #[serde(default)]
    pub exclude: Option<Vec<String>>,
}

/// IP safety configuration.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct IpSafety {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub owner: Option<String>,

    #[serde(default)]
    pub prohibited_attributions: Vec<String>,

    #[serde(default)]
    pub no_ai_co_author: bool,

    #[serde(default)]
    pub no_secrets: bool,

    #[serde(default)]
    pub no_internal_paths: bool,
}

/// Load config from a surfcontext.json file, or return defaults if missing.
pub fn load_config(repo_root: &Path) -> Result<SurfConfig> {
    let config_path = repo_root.join("surfcontext.json");

    if config_path.exists() {
        let raw = std::fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read {}", config_path.display()))?;
        let config: SurfConfig = serde_json::from_str(&raw)
            .with_context(|| format!("Failed to parse {}", config_path.display()))?;
        Ok(config)
    } else {
        // Return sensible defaults
        Ok(SurfConfig {
            version: "3.0".to_string(),
            platforms: vec!["claude".to_string()],
            canonical: Canonical::default(),
            generation: HashMap::new(),
            sync: HashMap::new(),
            discovery_order: Vec::new(),
            ip_safety: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_full_config() {
        let json = r#"{
            "version": "3.0",
            "platforms": ["claude", "codex"],
            "canonical": {
                "rootContext": "CONTEXT.md",
                "agentsDir": ".context/agents",
                "docsDir": ".context/docs",
                "skillsDir": ".context/skills",
                "guidesDir": ".context/guides"
            },
            "generation": {
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
            },
            "sync": {
                "skills": {
                    "source": ".context/skills",
                    "targets": [
                        { "repo": "../remote-flow-web", "dest": ".claude/skills" },
                        { "repo": "../wavesite", "dest": ".claude/skills", "include": ["check-deployment"] }
                    ]
                }
            },
            "discoveryOrder": ["CONTEXT.md", "surfcontext.json"],
            "ipSafety": {
                "enabled": true,
                "owner": "CloudSurf Software LLC",
                "noAiCoAuthor": true
            }
        }"#;

        let config: SurfConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.version, "3.0");
        assert_eq!(config.platforms.len(), 2);
        assert_eq!(config.canonical.agents_dir, ".context/agents");
        assert_eq!(config.generation.len(), 2);
        assert_eq!(
            config.generation["claude"].root_context.as_deref(),
            Some("CLAUDE.md")
        );
        assert_eq!(config.generation["codex"].method, "template-copy");
        assert!(config.sync.contains_key("skills"));
        assert_eq!(config.sync["skills"].targets.len(), 2);
        assert_eq!(
            config.sync["skills"].targets[1].include.as_ref().unwrap()[0],
            "check-deployment"
        );
        assert!(config.ip_safety.unwrap().no_ai_co_author);
    }

    #[test]
    fn test_parse_minimal_config() {
        let json = r#"{ "version": "3.0" }"#;
        let config: SurfConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.canonical.root_context, "CONTEXT.md");
        assert!(config.sync.is_empty());
    }

    #[test]
    fn test_defaults() {
        let json = r#"{}"#;
        let config: SurfConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.version, "3.0");
        assert_eq!(config.canonical.agents_dir, ".context/agents");
        assert_eq!(config.canonical.docs_dir, ".context/docs");
        assert_eq!(config.canonical.skills_dir, ".context/skills");
        assert_eq!(config.canonical.guides_dir, ".context/guides");
    }
}
