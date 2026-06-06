use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::audit::Vulnerability;
use crate::health::HealthResult;
use crate::outdated::OutdatedDep;
use crate::tree::DepTreeInfo;
use crate::unused::UnusedDep;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    pub crate_name: String,
    pub timestamp: String,
    pub vulnerabilities: Vec<Vulnerability>,
    pub outdated: Vec<OutdatedDep>,
    pub dep_tree: DepTreeInfo,
    pub unused: Vec<UnusedDep>,
    pub score: HealthResult,
}

impl AuditReport {
    pub fn write_json(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn write_markdown(&self, path: &Path) -> Result<()> {
        let mut md = String::new();
        md.push_str(&format!("# Audit Report: {}\n\n", self.crate_name));
        md.push_str(&format!("**Date:** {}\n\n", self.timestamp));
        md.push_str(&format!(
            "**Health Score:** {}/100 ({})\n\n",
            self.score.score, self.score.grade
        ));

        md.push_str("## Recommendations\n\n");
        for r in &self.score.recommendations {
            md.push_str(&format!("- {}\n", r));
        }

        std::fs::write(path, md)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::{HealthInput, HealthScore};

    fn make_report() -> AuditReport {
        let score = HealthScore::compute(&HealthInput {
            vulnerability_count: 0,
            outdated_count: 0,
            unused_count: 0,
            max_depth: 2,
            total_deps: 5,
        });
        AuditReport {
            crate_name: "test-crate".to_string(),
            timestamp: "2026-06-06T00:00:00Z".to_string(),
            vulnerabilities: vec![],
            outdated: vec![],
            dep_tree: DepTreeInfo {
                max_depth: 2,
                total_deps: 5,
                direct_deps: vec![],
                deep_deps: vec![],
            },
            unused: vec![],
            score,
        }
    }

    #[test]
    fn test_write_json() {
        let report = make_report();
        let dir = std::env::temp_dir().join("dep-audit-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.json");
        report.write_json(&path).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("test-crate"));
    }

    #[test]
    fn test_write_markdown() {
        let report = make_report();
        let dir = std::env::temp_dir().join("dep-audit-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.md");
        report.write_markdown(&path).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("# Audit Report: test-crate"));
    }
}
