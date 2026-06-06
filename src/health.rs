use serde::{Deserialize, Serialize};

/// Input for health score computation
#[derive(Debug, Clone)]
pub struct HealthInput {
    pub vulnerability_count: usize,
    pub outdated_count: usize,
    pub unused_count: usize,
    pub max_depth: usize,
    pub total_deps: usize,
}

/// Health score result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResult {
    pub score: u8,
    pub grade: String,
    pub breakdown: ScoreBreakdown,
    pub recommendations: Vec<String>,
}

/// Breakdown of individual score components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub vulnerability_penalty: i32,
    pub outdated_penalty: i32,
    pub unused_penalty: i32,
    pub depth_penalty: i32,
    pub bonus: i32,
}

/// Computes an overall health score (0-100).
pub struct HealthScore;

impl HealthScore {
    /// Compute health score from the given input.
    pub fn compute(input: &HealthInput) -> HealthResult {
        let mut score: i32 = 100;
        let mut recommendations = Vec::new();

        // Vulnerability penalty: -15 per vuln, severe penalty
        let vuln_penalty = (input.vulnerability_count as i32 * 15).min(60);
        score -= vuln_penalty;
        if input.vulnerability_count > 0 {
            recommendations.push(format!(
                "🚨 Fix {} vulnerabilities — run `cargo audit` for details",
                input.vulnerability_count
            ));
        }

        // Outdated penalty: -5 per outdated dep, capped at 30
        let outdated_penalty = (input.outdated_count as i32 * 5).min(30);
        score -= outdated_penalty;
        if input.outdated_count > 0 {
            recommendations.push(format!(
                "📦 Update {} outdated dependencies — consider `cargo update`",
                input.outdated_count
            ));
        }

        // Unused penalty: -3 per unused dep, capped at 20
        let unused_penalty = (input.unused_count as i32 * 3).min(20);
        score -= unused_penalty;
        if input.unused_count > 0 {
            recommendations.push(format!(
                "🧹 Remove {} unused dependencies from Cargo.toml",
                input.unused_count
            ));
        }

        // Depth penalty: >5 is concerning, >10 is bad
        let depth_penalty = if input.max_depth > 10 {
            10
        } else if input.max_depth > 5 {
            5
        } else {
            0
        };
        score -= depth_penalty;
        if input.max_depth > 5 {
            recommendations.push(format!(
                "🌳 Dependency tree depth is {} — consider reducing transitive deps",
                input.max_depth
            ));
        }

        // Bonus: zero vulns, zero unused
        let mut bonus = 0i32;
        if input.vulnerability_count == 0 {
            bonus += 5;
        }
        if input.unused_count == 0 {
            bonus += 3;
        }
        score += bonus;

        score = score.clamp(0, 100);

        let grade = match score {
            90..=100 => 'A',
            80..=89 => 'B',
            70..=79 => 'C',
            60..=69 => 'D',
            _ => 'F',
        };

        if score == 100 {
            recommendations.push("✅ Crate is in excellent health!".to_string());
        }

        HealthResult {
            score: score as u8,
            grade: grade.to_string(),
            breakdown: ScoreBreakdown {
                vulnerability_penalty: vuln_penalty,
                outdated_penalty,
                unused_penalty,
                depth_penalty,
                bonus,
            },
            recommendations,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perfect_score() {
        let input = HealthInput {
            vulnerability_count: 0,
            outdated_count: 0,
            unused_count: 0,
            max_depth: 2,
            total_deps: 5,
        };
        let score = HealthScore::compute(&input);
        assert_eq!(score.score, 100);
        assert_eq!(score.grade, "A");
    }

    #[test]
    fn test_vulnerability_penalty() {
        let input = HealthInput {
            vulnerability_count: 2,
            outdated_count: 0,
            unused_count: 0,
            max_depth: 2,
            total_deps: 5,
        };
        let score = HealthScore::compute(&input);
        assert!(score.score < 100);
        assert_eq!(score.breakdown.vulnerability_penalty, 30);
    }

    #[test]
    fn test_failing_score() {
        let input = HealthInput {
            vulnerability_count: 5,
            outdated_count: 10,
            unused_count: 8,
            max_depth: 12,
            total_deps: 100,
        };
        let score = HealthScore::compute(&input);
        assert!(score.score < 50);
        assert_eq!(score.grade, "F");
    }

    #[test]
    fn test_depth_penalty() {
        let deep = HealthInput {
            vulnerability_count: 0,
            outdated_count: 0,
            unused_count: 0,
            max_depth: 12,
            total_deps: 50,
        };
        let shallow = HealthInput {
            vulnerability_count: 0,
            outdated_count: 0,
            unused_count: 0,
            max_depth: 3,
            total_deps: 5,
        };
        let deep_score = HealthScore::compute(&deep);
        let shallow_score = HealthScore::compute(&shallow);
        assert!(deep_score.score < shallow_score.score);
    }

    #[test]
    fn test_bonus_for_clean() {
        let input = HealthInput {
            vulnerability_count: 0,
            outdated_count: 2,
            unused_count: 0,
            max_depth: 3,
            total_deps: 10,
        };
        let score = HealthScore::compute(&input);
        assert_eq!(score.breakdown.bonus, 8); // no vulns (5) + no unused (3)
    }
}
