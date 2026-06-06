use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

/// A single vulnerability found by cargo audit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vulnerability {
    pub id: String,
    pub package: String,
    pub version: String,
    pub severity: String,
    pub title: String,
    pub url: Option<String>,
}

/// Runs `cargo audit` and parses its output for vulnerabilities.
pub struct AuditScanner;

impl AuditScanner {
    /// Run cargo audit on the given crate path and return parsed vulnerabilities.
    pub fn run(crate_path: &Path) -> Result<Vec<Vulnerability>> {
        let output = Command::new("cargo")
            .args(["audit"])
            .current_dir(crate_path)
            .output()
            .context("failed to run cargo audit — is `cargo-audit` installed?")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{stdout}\n{stderr}");

        Ok(Self::parse_output(&combined))
    }

    /// Parse cargo audit text output into structured vulnerabilities.
    pub fn parse_output(text: &str) -> Vec<Vulnerability> {
        let mut vulns = Vec::new();
        // Pattern: ID    Title
        //         └── package: vVERSION [severity]
        let id_re = Regex::new(r"(?m)^([A-Z]+-\d+-\d+)\s+(.+)$").unwrap();
        let detail_re =
            Regex::new(r"(?m)└──\s+(\S+)\s+v(\S+)\s*(.*)$").unwrap();
        let url_re = Regex::new(r"https?://\S+").unwrap();

        let lines: Vec<&str> = text.lines().collect();
        let mut i = 0;
        while i < lines.len() {
            if let Some(caps) = id_re.captures(lines[i]) {
                let id = caps[1].to_string();
                let title = caps[2].trim().to_string();

                // Look for detail line
                let mut package = String::new();
                let mut version = String::new();
                let mut severity = String::new();
                let mut url = None;

                for j in (i + 1)..std::cmp::min(i + 5, lines.len()) {
                    if let Some(dc) = detail_re.captures(lines[j]) {
                        package = dc[1].to_string();
                        version = dc[2].to_string();
                        let rest = dc[3].to_string();
                        // Extract severity from brackets or classify
                        if rest.contains("critical") || rest.contains("high") {
                            severity = "high".to_string();
                        } else if rest.contains("medium") || rest.contains("moderate") {
                            severity = "medium".to_string();
                        } else if rest.contains("low") {
                            severity = "low".to_string();
                        } else {
                            severity = "unknown".to_string();
                        }
                    }
                    if let Some(m) = url_re.find(lines[j]) {
                        url = Some(m.as_str().to_string());
                    }
                }

                vulns.push(Vulnerability {
                    id,
                    package,
                    version,
                    severity,
                    title,
                    url,
                });
            }
            i += 1;
        }

        vulns
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_output() {
        let vulns = AuditScanner::parse_output("");
        assert!(vulns.is_empty());
    }

    #[test]
    fn test_parse_no_vulnerabilities() {
        let text = "    Scanning Cargo.lock...\n    No vulnerabilities found.";
        let vulns = AuditScanner::parse_output(text);
        assert!(vulns.is_empty());
    }

    #[test]
    fn test_parse_single_vulnerability() {
        let text = "RUSTSEC-2021-0139 Test vuln
└── ansi_term v0.12.1
   https://github.com/...";
        let vulns = AuditScanner::parse_output(text);
        assert_eq!(vulns.len(), 1);
        assert_eq!(vulns[0].id, "RUSTSEC-2021-0139");
        assert_eq!(vulns[0].title, "Test vuln");
        assert_eq!(vulns[0].package, "ansi_term");
        assert_eq!(vulns[0].version, "0.12.1");
    }

    #[test]
    fn test_parse_multiple_vulnerabilities() {
        let text = "RUSTSEC-2021-0139 First issue
└── pkg1 v1.0.0

RUSTSEC-2022-0045 Second issue
└── pkg2 v2.0.0";
        let vulns = AuditScanner::parse_output(text);
        assert_eq!(vulns.len(), 2);
        assert_eq!(vulns[0].id, "RUSTSEC-2021-0139");
        assert_eq!(vulns[1].id, "RUSTSEC-2022-0045");
    }
}
