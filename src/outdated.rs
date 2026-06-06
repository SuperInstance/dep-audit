use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

/// An outdated dependency with current and latest version info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutdatedDep {
    pub name: String,
    pub current: String,
    pub latest: String,
    pub kind: String, // "normal", "dev", "build"
}

/// Checks for outdated dependencies.
pub struct DepOutdated;

impl DepOutdated {
    /// Check outdated deps by running `cargo outdated` or comparing with crates.io
    pub fn check(crate_path: &Path) -> Result<Vec<OutdatedDep>> {
        // First try cargo outdated
        if let Ok(output) = Command::new("cargo")
            .args(["outdated", "--format", "json"])
            .current_dir(crate_path)
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Ok(deps) = Self::parse_cargo_outdated_json(&stdout) {
                    return Ok(deps);
                }
            }
        }

        // Fallback: parse Cargo.toml and check crates.io
        Self::check_via_crates_io(crate_path)
    }

    /// Parse cargo outdated JSON output
    pub fn parse_cargo_outdated_json(json: &str) -> Result<Vec<OutdatedDep>> {
        let parsed: Vec<HashMap<String, String>> = serde_json::from_str(json.trim()).unwrap_or_default();
        let mut deps = Vec::new();
        for entry in parsed {
            let name = entry.get("name").cloned().unwrap_or_default();
            let current = entry.get("current").cloned().unwrap_or_default();
            let latest = entry.get("latest").cloned().unwrap_or_default();
            let kind = entry.get("kind").cloned().unwrap_or_else(|| "normal".to_string());
            if !name.is_empty() && current != latest && !latest.is_empty() {
                deps.push(OutdatedDep {
                    name,
                    current,
                    latest,
                    kind,
                });
            }
        }
        Ok(deps)
    }

    /// Parse Cargo.toml and check each dep against crates.io
    fn check_via_crates_io(crate_path: &Path) -> Result<Vec<OutdatedDep>> {
        let cargo_toml = std::fs::read_to_string(crate_path.join("Cargo.toml"))
            .context("failed to read Cargo.toml")?;
        let manifest: toml::Value = toml::from_str(&cargo_toml)
            .context("failed to parse Cargo.toml")?;

        let mut outdated = Vec::new();

        let deps_table = manifest
            .get("dependencies")
            .and_then(|v| v.as_table());

        if let Some(deps) = deps_table {
            for (name, value) in deps {
                if let Some(current) = Self::extract_version(value) {
                    if let Some(latest) = Self::fetch_latest_version(name) {
                        if current != latest {
                            outdated.push(OutdatedDep {
                                name: name.clone(),
                                current,
                                latest,
                                kind: "normal".to_string(),
                            });
                        }
                    }
                }
            }
        }

        let dev_deps = manifest
            .get("dev-dependencies")
            .and_then(|v| v.as_table());

        if let Some(deps) = dev_deps {
            for (name, value) in deps {
                if let Some(current) = Self::extract_version(value) {
                    if let Some(latest) = Self::fetch_latest_version(name) {
                        if current != latest {
                            outdated.push(OutdatedDep {
                                name: name.clone(),
                                current,
                                latest,
                                kind: "dev".to_string(),
                            });
                        }
                    }
                }
            }
        }

        Ok(outdated)
    }

    /// Extract version string from a dependency value
    pub fn extract_version(value: &toml::Value) -> Option<String> {
        match value {
            toml::Value::String(s) => {
                // Strip version operators
                let v = s
                    .trim_start_matches(|c: char| c == '^' || c == '~' || c == '=' || c == '>' || c == '<')
                    .split(',')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if v.contains('.') && !v.is_empty() {
                    Some(v)
                } else {
                    None
                }
            }
            toml::Value::Table(t) => t
                .get("version")
                .and_then(|v| v.as_str())
                .map(|s| {
                    s.trim_start_matches(|c: char| c == '^' || c == '~' || c == '=')
                        .split(',')
                        .next()
                        .unwrap_or("")
                        .trim()
                        .to_string()
                }),
            _ => None,
        }
    }

    /// Fetch latest version from crates.io
    fn fetch_latest_version(crate_name: &str) -> Option<String> {
        let url = format!("https://crates.io/api/v1/crates/{crate_name}");
        let output = Command::new("curl")
            .args(["-s", "-H", "User-Agent: dep-audit/0.1", &url])
            .output()
            .ok()?;

        let body = String::from_utf8_lossy(&output.stdout);
        let parsed: serde_json::Value = serde_json::from_str(&body).ok()?;
        parsed
            .get("crate")
            .and_then(|c| c.get("max_version"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_version_string() {
        let v = toml::Value::String("1.0".to_string());
        assert_eq!(DepOutdated::extract_version(&v), Some("1.0".to_string()));
    }

    #[test]
    fn test_extract_version_caret() {
        let v = toml::Value::String("^0.10".to_string());
        assert_eq!(DepOutdated::extract_version(&v), Some("0.10".to_string()));
    }

    #[test]
    fn test_extract_version_table() {
        let mut t = toml::map::Map::new();
        t.insert("version".to_string(), toml::Value::String("1".to_string()));
        let v = toml::Value::Table(t);
        assert_eq!(DepOutdated::extract_version(&v), Some("1".to_string()));
    }

    #[test]
    fn test_extract_version_git_dep() {
        let mut t = toml::map::Map::new();
        t.insert("git".to_string(), toml::Value::String("https://...".to_string()));
        let v = toml::Value::Table(t);
        assert_eq!(DepOutdated::extract_version(&v), None);
    }

    #[test]
    fn test_parse_empty_json() {
        let deps = DepOutdated::parse_cargo_outdated_json("[]").unwrap();
        assert!(deps.is_empty());
    }
}
