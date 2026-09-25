use crate::eval::types::{OracleCheck, TextOracle, ToolExecution, ToolOracle};

pub fn judge_text(oracle: &TextOracle, response: &str) -> (Vec<OracleCheck>, Option<f64>) {
    let normalize = |value: &str| {
        if oracle.case_sensitive {
            value.to_string()
        } else {
            value.to_lowercase()
        }
    };
    let response = normalize(response);
    let mut checks = Vec::new();
    for needle in &oracle.required_all {
        checks.push(OracleCheck {
            rule: format!("contains:{needle}"),
            passed: response.contains(&normalize(needle)),
        });
    }
    for group in &oracle.required_any {
        checks.push(OracleCheck {
            rule: format!("contains_any:{}", group.join("|")),
            passed: group
                .iter()
                .any(|needle| response.contains(&normalize(needle))),
        });
    }
    for needle in &oracle.forbidden {
        checks.push(OracleCheck {
            rule: format!("forbids:{needle}"),
            passed: !response.contains(&normalize(needle)),
        });
    }
    if let Some(limit) = oracle.max_response_chars {
        checks.push(OracleCheck {
            rule: format!("max_response_chars:{limit}"),
            passed: response.chars().count() <= limit,
        });
    }
    let score = (!checks.is_empty()).then(|| {
        let passed = checks.iter().filter(|check| check.passed).count();
        passed as f64 * 100.0 / checks.len() as f64
    });
    (checks, score)
}

pub fn judge_tools(oracle: &ToolOracle, executions: &[ToolExecution]) -> Vec<OracleCheck> {
    let mut checks = Vec::new();
    for tool in &oracle.required_tools {
        checks.push(OracleCheck {
            rule: format!("executes:{tool}"),
            passed: executions
                .iter()
                .any(|execution| execution.tool == *tool && execution.status == "ok"),
        });
    }
    for tool in &oracle.forbidden_tools {
        checks.push(OracleCheck {
            rule: format!("forbids_tool:{tool}"),
            passed: !executions.iter().any(|execution| execution.tool == *tool),
        });
    }
    if let Some(minimum) = oracle.min_executions {
        checks.push(OracleCheck {
            rule: format!("min_tool_executions:{minimum}"),
            passed: executions.len() >= minimum,
        });
    }
    if let Some(maximum) = oracle.max_executions {
        checks.push(OracleCheck {
            rule: format!("max_tool_executions:{maximum}"),
            passed: executions.len() <= maximum,
        });
    }
    checks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_oracle_scores_required_and_forbidden_rules() {
        let oracle = TextOracle {
            case_sensitive: false,
            required_all: vec!["argv".into()],
            required_any: vec![vec!["proof".into(), "evidence".into()]],
            forbidden: vec!["sh -c".into()],
            max_response_chars: Some(100),
        };
        let (_, score) = judge_text(&oracle, "ARGV with proof");
        assert_eq!(score, Some(100.0));
    }
}
