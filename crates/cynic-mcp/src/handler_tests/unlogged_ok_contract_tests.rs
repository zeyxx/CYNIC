#[cfg(test)]
mod unlogged_ok_contract_tests {
    use super::*;

    #[test]
    fn unlogged_ok_contract_rejects_unlogged_ok_call() {
        let temp = std::env::temp_dir().join(format!(
            "cynic-mcp-unlogged-ok-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp).expect("fixture directory should be created");
        std::fs::write(
            temp.join("bad.rs"),
            "fn test() {\n    let _ = thing.ok();\n}\n",
        )
        .expect("fixture file should be written");

        let findings = unlogged_ok_findings_for_files(
            temp.to_str().expect("temp path should be utf-8"),
            &["bad.rs".to_string()],
        );
        let _ = std::fs::remove_dir_all(&temp);

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str()) == Some("unlogged_ok_call")
        }));
    }

    #[test]
    fn unlogged_ok_contract_allows_logged_or_exempt_ok_call() {
        let temp = std::env::temp_dir().join(format!(
            "cynic-mcp-unlogged-ok-ok-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp).expect("fixture directory should be created");
        std::fs::write(
            temp.join("good.rs"),
            "fn test() {\n    tracing::warn!(\"x\");\n    let _ = thing.ok();\n    let _ = env::var(\"A\").ok();\n}\n",
        )
        .expect("fixture file should be written");

        let findings = unlogged_ok_findings_for_files(
            temp.to_str().expect("temp path should be utf-8"),
            &["good.rs".to_string()],
        );
        let _ = std::fs::remove_dir_all(&temp);

        assert!(findings.is_empty());
    }
}

