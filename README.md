# dep-audit

![License](https://img.shields.io/badge/license-MIT-blue)
![Language](https://img.shields.io/badge/language-Rust-orange)
![Part of SuperInstance](https://img.shields.io/badge/part%20of-SuperInstance-blue)

Audit Rust crate dependencies for vulnerabilities, outdated versions, tree depth, unused entries, and an overall health score — one command, full picture.

## Overview

Keeping a Rust project's dependency tree healthy means tracking vulnerabilities (RUSTSEC advisories), knowing what's outdated, spotting crates you imported but never used, and watching your transitive dependency depth. `dep-audit` runs all five checks against any Rust crate and produces a report in JSON, Markdown, or both.

Built for the SuperInstance monorepo's 562 crates, where a single vulnerability in a shared dependency can cascade across hundreds of packages.

## Installation

```bash
cargo install --path .
```

Requires `cargo-audit` and `cargo-outdated` for full results (graceful fallback if either is missing).

## Usage

```bash
# Full audit of a crate
dep-audit --path ./my-crate

# JSON report only
dep-audit --path ./my-crate --format json

# Markdown report, custom output directory
dep-audit --path ./my-crate --format markdown --output ./reports/

# Skip cargo audit (if not installed)
dep-audit --path ./my-crate --skip-audit

# Verbose — show everything
dep-audit --path ./my-crate --verbose
```

Output:

```
🔍 Auditing my-crate ...
  🛡  Running cargo audit ...
     Found 2 vulnerabilities
  📦 Checking for outdated dependencies ...
     Found 3 outdated deps
  🌳 Analyzing dependency tree ...
     Max depth: 7
  🧹 Checking for unused dependencies ...
     Found 1 unused dep

📊 Health Score: 72/100 (C)
```

## Architecture

```
dep-audit/
├── src/main.rs        CLI entry point, orchestrates the five audit stages
├── src/audit.rs       AuditScanner: runs cargo audit, parses RUSTSEC advisories
├── src/outdated.rs     DepOutdated: checks for outdated deps via cargo outdated or crates.io
├── src/tree.rs         TreeAnalyzer: dependency tree depth via cargo_metadata
├── src/unused.rs       UnusedChecker: detects deps in Cargo.toml not referenced in src/
└── src/health.rs       HealthScore: computes 0–100 score with letter grade
└── src/report.rs       ReportWriter: outputs JSON and/or Markdown
```

```
           ┌──────────────┐
           │  Cargo.toml   │
           │  + src/       │
           └──────┬────────┘
                  │
    ┌─────────────┼─────────────────┐
    │             │                  │
    ▼             ▼                  ▼
┌────────┐  ┌──────────┐  ┌──────────────┐
│ Audit  │  │ Outdated │  │  Tree Depth  │
│Scanner │  │ Checker  │  │  Analyzer    │
│(RUSTSEC│  │(versions) │  │  (metadata)  │
│ parse) │  │           │  │              │
└───┬────┘  └─────┬─────┘  └──────┬───────┘
    │             │                │
    │       ┌─────▼──────┐        │
    │       │   Unused   │        │
    │       │  Checker   │        │
    │       │(src scan)  │        │
    │       └─────┬──────┘        │
    │             │                │
    └─────────────┼────────────────┘
                  ▼
         ┌──────────────┐
         │  HealthScore │  0–100 + letter grade
         │  compute()   │
         └──────┬───────┘
                ▼
        ┌──────────────┐
        │ ReportWriter │  JSON + Markdown
        └──────────────┘
```

## API Reference

### `audit::AuditScanner`

```rust
pub struct AuditScanner;

impl AuditScanner {
    pub fn run(crate_path: &Path) -> Result<Vec<Vulnerability>>;
    pub fn parse_output(text: &str) -> Vec<Vulnerability>;
}

pub struct Vulnerability {
    pub id: String,        // e.g. "RUSTSEC-2021-0139"
    pub package: String,
    pub version: String,
    pub severity: String,  // "high", "medium", "low", "unknown"
    pub title: String,
    pub url: Option<String>,
}
```

### `outdated::DepOutdated`

```rust
pub struct DepOutdated;

impl DepOutdated {
    pub fn check(crate_path: &Path) -> Result<Vec<OutdatedDep>>;
}
```

Falls back from `cargo outdated` to direct crates.io comparison.

### `health::HealthScore`

```rust
pub struct HealthScore;

impl HealthScore {
    pub fn compute(input: &HealthInput) -> HealthResult;
}

pub struct HealthInput {
    pub vulnerability_count: usize,
    pub outdated_count: usize,
    pub unused_count: usize,
    pub max_depth: usize,
    pub total_deps: usize,
}

pub struct HealthResult {
    pub score: u8,           // 0–100
    pub grade: String,       // "A" through "F"
    pub breakdown: ScoreBreakdown,
    pub recommendations: Vec<String>,
}
```

### Scoring breakdown

| Factor | Penalty | Cap |
|--------|---------|-----|
| Vulnerabilities | -15 per | 60 |
| Outdated deps | -5 per | 30 |
| Unused deps | -3 per | 20 |
| Tree depth > 10 | -10 | — |
| Tree depth > 5 | -5 | — |
| Zero vulns bonus | +5 | — |
| Zero unused bonus | +3 | — |

Letter grades: A (90–100), B (80–89), C (70–79), D (60–69), F (< 60).

## CI Integration

```yaml
# .github/workflows/audit.yml
- name: Dependency Audit
  run: |
    cargo install --path /path/to/dep-audit
    dep-audit --path . --format json --output ./audit-reports --skip-audit
    # Fail on score below B
    SCORE=$(jq '.health.score' audit-reports/*.json)
    test "$SCORE" -ge 80
```

## Related Crates

- **cross-compile-checker** — cross-platform compatibility analysis
- **fleet-dedup** — duplicate repo detection across the fleet
- **ternary-pack** — dependency health matters more when packing ternary data
- **open-parallel** — fleet-wide dependency coordination

## License

MIT
