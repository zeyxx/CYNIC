#[cfg(test)]
mod pipeline_functions_contract_tests {
    use super::*;

    #[test]
    fn pipeline_functions_contract_rejects_reimplementation() {
        let temp = std::env::temp_dir().join(format!(
            "cynic-mcp-pipeline-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp).expect("fixture directory should be created");
        std::fs::write(
            temp.join("bad.rs"),
            "fn compute_qscore(x: i32) -> i32 { x }\n",
        )
        .expect("fixture file should be written");

        let findings = pipeline_function_findings(
            temp.as_path(),
            temp.to_str().expect("temp path should be utf-8"),
        );
        let _ = std::fs::remove_dir_all(&temp);

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str())
                == Some("reimplemented_pipeline_function")
        }));
    }

    #[test]
    fn pipeline_functions_contract_ignores_comments() {
        let temp = std::env::temp_dir().join(format!(
            "cynic-mcp-pipeline-ok-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp).expect("fixture directory should be created");
        std::fs::write(
            temp.join("good.rs"),
            "// fn compute_qscore(x: i32) -> i32 { x }\n",
        )
        .expect("fixture file should be written");

        let findings = pipeline_function_findings(
            temp.as_path(),
            temp.to_str().expect("temp path should be utf-8"),
        );
        let _ = std::fs::remove_dir_all(&temp);

        assert!(findings.is_empty());
    }
}

