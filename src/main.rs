use clap::Parser;
use std::path::PathBuf;

mod audit;
mod health;
mod outdated;
mod report;
mod tree;
mod unused;

/// Audit Rust crate dependencies: vulnerabilities, outdated deps, tree depth, unused deps
#[derive(Parser, Debug)]
#[command(name = "dep-audit", version, about)]
struct Args {
    /// Path to the Rust crate to audit
    #[arg(short, long)]
    path: PathBuf,

    /// Output format: json, markdown, or both
    #[arg(short, long, default_value = "both")]
    format: String,

    /// Output directory for reports
    #[arg(short, long, default_value = ".")]
    output: PathBuf,

    /// Skip cargo audit (if not installed)
    #[arg(long)]
    skip_audit: bool,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if !args.path.exists() {
        anyhow::bail!("Path does not exist: {}", args.path.display());
    }
    if !args.path.join("Cargo.toml").exists() {
        anyhow::bail!("No Cargo.toml found at: {}", args.path.display());
    }

    let crate_path = args.path.canonicalize()?;
    let crate_name = crate_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    eprintln!("🔍 Auditing {} ...", crate_name);

    // 1. AuditScanner — vulnerabilities
    let vulnerabilities = if args.skip_audit {
        eprintln!("  ⏭  Skipping cargo audit");
        Vec::new()
    } else {
        eprintln!("  🛡  Running cargo audit ...");
        match audit::AuditScanner::run(&crate_path) {
            Ok(vulns) => {
                eprintln!("     Found {} vulnerabilities", vulns.len());
                vulns
            }
            Err(e) => {
                eprintln!("     ⚠  cargo audit failed: {e}");
                Vec::new()
            }
        }
    };

    // 2. DepOutdated — outdated deps
    eprintln!("  📦 Checking for outdated dependencies ...");
    let outdated = outdated::DepOutdated::check(&crate_path).unwrap_or_else(|e| {
        eprintln!("     ⚠  Outdated check failed: {e}");
        Vec::new()
    });
    eprintln!("     Found {} outdated deps", outdated.len());

    // 3. DepTree — dependency tree
    eprintln!("  🌳 Building dependency tree ...");
    let dep_tree = tree::DepTree::build(&crate_path).unwrap_or_else(|e| {
        eprintln!("     ⚠  Tree build failed: {e}");
        tree::DepTreeInfo::default()
    });
    eprintln!(
        "     Depth: {}, Total deps: {}",
        dep_tree.max_depth, dep_tree.total_deps
    );

    // 4. UnusedDetector
    eprintln!("  🧹 Detecting unused dependencies ...");
    let unused = unused::UnusedDetector::detect(&crate_path).unwrap_or_else(|e| {
        eprintln!("     ⚠  Unused detection failed: {e}");
        Vec::new()
    });
    eprintln!("     Found {} unused deps", unused.len());

    // 5. HealthScore
    let score = health::HealthScore::compute(&health::HealthInput {
        vulnerability_count: vulnerabilities.len(),
        outdated_count: outdated.len(),
        unused_count: unused.len(),
        max_depth: dep_tree.max_depth,
        total_deps: dep_tree.total_deps,
    });
    eprintln!("  ❤️  Health Score: {}/100", score.score);

    // 6. AuditReport
    let report_data = report::AuditReport {
        crate_name: crate_name.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        vulnerabilities,
        outdated,
        dep_tree,
        unused,
        score,
    };

    std::fs::create_dir_all(&args.output)?;

    match args.format.as_str() {
        "json" => {
            let path = args.output.join(format!("{crate_name}-audit.json"));
            report_data.write_json(&path)?;
            println!("📄 JSON report: {}", path.display());
        }
        "markdown" | "md" => {
            let path = args.output.join(format!("{crate_name}-audit.md"));
            report_data.write_markdown(&path)?;
            println!("📄 Markdown report: {}", path.display());
        }
        _ => {
            let json_path = args.output.join(format!("{crate_name}-audit.json"));
            let md_path = args.output.join(format!("{crate_name}-audit.md"));
            report_data.write_json(&json_path)?;
            report_data.write_markdown(&md_path)?;
            println!("📄 JSON report: {}", json_path.display());
            println!("📄 Markdown report: {}", md_path.display());
        }
    }

    Ok(())
}
