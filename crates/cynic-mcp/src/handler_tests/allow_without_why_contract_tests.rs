#[cfg(test)]
mod allow_without_why_contract_tests {
    use super::*;

    #[test]
    fn allow_without_why_contract_rejects_allow_without_context() {
        let temp = std::env::temp_dir().join(format!(
            "cynic-mcp-allow-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos()
        ));
        let kernel_dir = temp.join("crates/cynic-kernel/src");
        std::fs::create_dir_all(&kernel_dir).expect("fixture directory should be created");
        std::fs::write(
            kernel_dir.join("bad.rs"),
            "#[allow(clippy::too_many_arguments)]\nfn x() {}\n",
        )
        .expect("fixture file should be written");

        let findings =
            validate_allow_without_why(temp.to_str().expect("temp path should be utf-8"));
        let _ = std::fs::remove_dir_all(&temp);

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str()) == Some("allow_without_why")
        }));
    }

    #[test]
    fn allow_without_why_contract_accepts_nearby_why_comment() {
        let temp = std::env::temp_dir().join(format!(
            "cynic-mcp-allow-ok-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos()
        ));
        let kernel_dir = temp.join("crates/cynic-kernel/src");
        std::fs::create_dir_all(&kernel_dir).expect("fixture directory should be created");
        std::fs::write(
            kernel_dir.join("good.rs"),
            "// WHY: justified\n#[allow(clippy::too_many_arguments)]\nfn x() {}\n",
        )
        .expect("fixture file should be written");

        let findings =
            validate_allow_without_why(temp.to_str().expect("temp path should be utf-8"));
        let _ = std::fs::remove_dir_all(&temp);

        assert!(findings.is_empty());
    }
}

