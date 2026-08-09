#[cfg(test)]
mod hardcoded_paths_contract_tests {
    use super::*;

    #[test]
    fn hardcoded_paths_contract_rejects_home_path_usage() {
        let temp = std::env::temp_dir().join(format!(
            "cynic-mcp-hardcoded-paths-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp).expect("fixture directory should be created");
        let file = temp.join("example.py");
        let home_path = format!("/{}/user/project", "home");
        std::fs::write(&file, format!("ROOT = \"{home_path}\"\n"))
            .expect("fixture file should be written");

        let findings = hardcoded_path_findings_for_files(
            temp.to_str().expect("temp path should be utf-8"),
            &["example.py"],
        );
        let _ = std::fs::remove_dir_all(&temp);

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str()) == Some("hardcoded_absolute_path")
        }));
    }

    #[test]
    fn hardcoded_paths_contract_accepts_relative_or_env_paths() {
        let temp = std::env::temp_dir().join(format!(
            "cynic-mcp-hardcoded-paths-ok-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp).expect("fixture directory should be created");
        let file = temp.join("example.py");
        std::fs::write(&file, "ROOT = os.getenv(\"CYNIC_CONFIG_ROOT\")\n")
            .expect("fixture file should be written");

        let findings = hardcoded_path_findings_for_files(
            temp.to_str().expect("temp path should be utf-8"),
            &["example.py"],
        );
        let _ = std::fs::remove_dir_all(&temp);

        assert!(findings.is_empty());
    }
}

