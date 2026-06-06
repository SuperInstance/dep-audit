use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Dependency tree information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DepTreeInfo {
    pub max_depth: usize,
    pub total_deps: usize,
    pub direct_deps: Vec<String>,
    pub deep_deps: Vec<DeepDep>,
}

/// A deep (transitive) dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepDep {
    pub name: String,
    pub depth: usize,
}

/// Builds the full dependency tree.
pub struct DepTree;

impl DepTree {
    /// Build dependency tree using cargo_metadata
    pub fn build(crate_path: &Path) -> Result<DepTreeInfo> {
        let manifest_path = crate_path.join("Cargo.toml");
        let mut cmd = cargo_metadata::MetadataCommand::new();
        cmd.manifest_path(&manifest_path);

        let metadata = cmd.exec().context("failed to resolve cargo metadata")?;

        let resolve = metadata
            .resolve
            .as_ref()
            .context("cargo metadata did not return resolve graph")?;

        let root_id = resolve
            .root
            .as_ref()
            .context("no root package in resolve graph")?;

        // Build adjacency list
        let mut adj: std::collections::HashMap<&str, Vec<&str>> =
            std::collections::HashMap::new();
        for node in &resolve.nodes {
            let id_str = node.id.repr.split(' ').next().unwrap_or(&node.id.repr);
            let deps: Vec<&str> = node
                .dependencies
                .iter()
                .map(|d| d.repr.split(' ').next().unwrap_or(&d.repr))
                .collect();
            adj.insert(id_str, deps);
        }

        let root_str = root_id.repr.split(' ').next().unwrap_or(&root_id.repr);
        let direct_deps = adj.get(root_str).cloned().unwrap_or_default();

        // BFS to compute depths
        let mut depths: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back((root_str, 0usize));

        while let Some((node, depth)) = queue.pop_front() {
            if depths.contains_key(node) {
                continue;
            }
            depths.insert(node, depth);
            if let Some(deps) = adj.get(node) {
                for dep in deps {
                    if !depths.contains_key(dep) {
                        queue.push_back((dep, depth + 1));
                    }
                }
            }
        }

        let max_depth = depths.values().copied().max().unwrap_or(0);
        let total_deps = depths.len().saturating_sub(1); // exclude root

        // Find deep deps (depth > 2)
        let deep_deps: Vec<DeepDep> = depths
            .iter()
            .filter(|(_, &d)| d > 2)
            .map(|(&name, &depth)| DeepDep {
                name: Self::extract_crate_name(name),
                depth,
            })
            .collect();

        let direct_deps: Vec<String> = direct_deps
            .iter()
            .map(|d| Self::extract_crate_name(d))
            .collect();

        Ok(DepTreeInfo {
            max_depth,
            total_deps,
            direct_deps,
            deep_deps,
        })
    }

    /// Extract crate name from node id like "name v0.1.0 (path)" or registry source
    fn extract_crate_name(id: &str) -> String {
        // Format: "name version source" — take the first word
        id.split_whitespace()
            .next()
            .unwrap_or(id)
            .to_string()
    }

    /// Parse cargo tree output as fallback
    pub fn parse_cargo_tree_output(output: &str) -> DepTreeInfo {
        let mut max_depth = 0usize;
        let mut all_deps = std::collections::HashSet::new();

        for line in output.lines() {
            if line.trim().is_empty() {
                continue;
            }
            // Count indentation level (groups of 4 spaces or │/├/└ chars)
            let depth = line
                .chars()
                .take_while(|c| c.is_whitespace() || *c == '│' || *c == '├' || *c == '└' || *c == '─')
                .count()
                / 2;
            let name = line
                .trim()
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_string();
            if !name.is_empty() {
                all_deps.insert(name);
                max_depth = max_depth.max(depth);
            }
        }

        DepTreeInfo {
            max_depth,
            total_deps: all_deps.len(),
            direct_deps: Vec::new(),
            deep_deps: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_tree() {
        let info = DepTree::parse_cargo_tree_output("");
        assert_eq!(info.total_deps, 0);
    }

    #[test]
    fn test_parse_simple_tree() {
        let output = "dep-audit v0.1.0
├── clap v4.0
│   └── anstream v0.6
└── serde v1.0";
        let info = DepTree::parse_cargo_tree_output(output);
        assert!(info.total_deps >= 3);
        assert!(info.max_depth >= 1);
    }

    #[test]
    fn test_extract_crate_name() {
        assert_eq!(DepTree::extract_crate_name("serde v1.0.100"), "serde");
        assert_eq!(DepTree::extract_crate_name("clap v4.0.0"), "clap");
    }
}
