use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

/// An unused dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnusedDep {
    pub name: String,
    pub kind: String, // "normal", "dev", "build"
    pub suggestion: String,
}

/// Detects dependencies listed in Cargo.toml but not actually imported in code.
pub struct UnusedDetector;

impl UnusedDetector {
    /// Detect unused dependencies in the given crate.
    pub fn detect(crate_path: &Path) -> Result<Vec<UnusedDep>> {
        let cargo_toml = std::fs::read_to_string(crate_path.join("Cargo.toml"))
            .context("failed to read Cargo.toml")?;
        let manifest: toml::Value = toml::from_str(&cargo_toml)?;

        let src_dir = crate_path.join("src");
        let src_files = Self::collect_rust_files(&src_dir)?;

        let source_content: String = src_files
            .iter()
            .filter_map(|p| std::fs::read_to_string(p).ok())
            .collect::<Vec<_>>()
            .join("\n");

        let mut unused = Vec::new();

        // Check normal dependencies
        if let Some(deps) = manifest.get("dependencies").and_then(|v| v.as_table()) {
            for name in deps.keys() {
                if !Self::is_used(name, &source_content) {
                    unused.push(UnusedDep {
                        name: name.clone(),
                        kind: "normal".to_string(),
                        suggestion: format!("Remove `{name}` from [dependencies] in Cargo.toml"),
                    });
                }
            }
        }

        // Check build dependencies
        if let Some(deps) = manifest.get("build-dependencies").and_then(|v| v.as_table()) {
            for name in deps.keys() {
                if !Self::is_used(name, &source_content) {
                    unused.push(UnusedDep {
                        name: name.clone(),
                        kind: "build".to_string(),
                        suggestion: format!("Remove `{name}` from [build-dependencies] in Cargo.toml"),
                    });
                }
            }
        }

        // Dev deps are not checked — they're typically used only in tests
        // which may be outside src/

        Ok(unused)
    }

    /// Collect all .rs files recursively
    fn collect_rust_files(dir: &Path) -> Result<Vec<std::path::PathBuf>> {
        let mut files = Vec::new();
        if !dir.exists() {
            return Ok(files);
        }
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                files.extend(Self::collect_rust_files(&path)?);
            } else if path.extension().map_or(false, |e| e == "rs") {
                files.push(path);
            }
        }
        Ok(files)
    }

    /// Check if a crate name is used in source code via `use` or `extern crate` or direct reference
    pub fn is_used(crate_name: &str, source: &str) -> bool {
        // Map common crate name variants
        let rust_ident = crate_name.replace('-', "_");

        // Check for `use crate_name`, `extern crate crate_name`, or `crate_name::`
        let patterns = [
            format!(r"use\s+{rust_ident}"),
            format!(r"extern\s+crate\s+{rust_ident}"),
            format!(r"{rust_ident}::"),
            // Also check for derive macros like #[serde(...)]
            format!(r"#\[{rust_ident}"),
            // Attribute usage like #[clap(...)]
            format!(r"#\[[^\]]*\b{rust_ident}\b"),
        ];

        for pat in &patterns {
            if let Ok(re) = Regex::new(pat) {
                if re.is_match(source) {
                    return true;
                }
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_used_import() {
        let source = "use serde::Deserialize;";
        assert!(UnusedDetector::is_used("serde", source));
    }

    #[test]
    fn test_is_used_extern() {
        let source = "extern crate regex;";
        assert!(UnusedDetector::is_used("regex", source));
    }

    #[test]
    fn test_is_used_path() {
        let source = "clap::Parser::parse();";
        assert!(UnusedDetector::is_used("clap", source));
    }

    #[test]
    fn test_is_used_derive() {
        let source = "#[serde(rename = \"foo\")]\nstruct Foo {}";
        assert!(UnusedDetector::is_used("serde", source));
    }

    #[test]
    fn test_is_not_used() {
        let source = "fn main() { println!(\"hello\"); }";
        assert!(!UnusedDetector::is_used("serde", source));
    }

    #[test]
    fn test_hyphenated_crate() {
        let source = "use cargo_metadata::MetadataCommand;";
        assert!(UnusedDetector::is_used("cargo-metadata", source));
    }
}
