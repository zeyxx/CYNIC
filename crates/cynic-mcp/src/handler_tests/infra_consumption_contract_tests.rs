#[cfg(test)]
mod infra_consumption_contract_tests {
    use super::*;

    fn temp_fixture_dir(name: &str) -> std::path::PathBuf {
        let unique = format!(
            "cynic-mcp-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos()
        );
        let dir = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&dir).expect("fixture directory should be created");
        dir
    }

    #[test]
    fn infra_consumption_contract_accepts_expected_contract_shape() {
        let contract = serde_json::json!({
            "schema": "cynic.infra_consumption.v1",
            "status": "normative",
            "capability_roles": ["build_runner"],
            "required_ports": ["LifecycleDispatchPort"],
            "native_lifecycle": {"forbidden_on_cortex": ["cargo build -p cynic-mcp"]},
            "service_placement": {"surrealdb": {"role": "storage_primary"}},
            "scan_scope": ["Makefile"],
            "forbidden_patterns": [
                {
                    "id": "cortex_local_mcp_build",
                    "pattern": "cargo build -p cynic-mcp",
                    "severity": "critical",
                    "detail": "MCP builds must be lifecycle-dispatched."
                }
            ]
        });

        assert_eq!(
            infra_consumption_contract_findings(
                "/tmp",
                "axioms/infra-consumption-v1.json",
                &contract
            ),
            Vec::<serde_json::Value>::new()
        );
    }

    #[test]
    fn infra_consumption_contract_rejects_uncovered_forbidden_on_cortex_rule() {
        let contract = serde_json::json!({
            "schema": "cynic.infra_consumption.v1",
            "status": "normative",
            "capability_roles": ["build_runner"],
            "required_ports": ["LifecycleDispatchPort"],
            "native_lifecycle": {"forbidden_on_cortex": ["rsync target/release/cynic-kernel"]},
            "service_placement": {"surrealdb": {"role": "storage_primary"}},
            "scan_scope": ["Makefile"],
            "forbidden_patterns": [
                {
                    "id": "cortex_local_mcp_build",
                    "pattern": "cargo build -p cynic-mcp",
                    "severity": "critical",
                    "detail": "MCP builds must be lifecycle-dispatched."
                }
            ]
        });

        let findings = infra_consumption_contract_findings(
            "/tmp",
            "axioms/infra-consumption-v1.json",
            &contract,
        );

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str())
                == Some("infra_consumption_forbidden_on_cortex_uncovered")
        }));
    }

    #[test]
    fn infra_consumption_contract_rejects_forbidden_cortex_local_mcp_build() {
        let temp = temp_fixture_dir("infra-consumption-mcp-build");
        let scanned = temp.join("wrapper.sh");
        std::fs::write(&scanned, "cargo build -p cynic-mcp --release\n")
            .expect("fixture file should be written");
        let contract = serde_json::json!({
            "schema": "cynic.infra_consumption.v1",
            "status": "normative",
            "capability_roles": ["build_runner"],
            "required_ports": ["LifecycleDispatchPort"],
            "native_lifecycle": {"forbidden_on_cortex": ["cargo build -p cynic-mcp"]},
            "service_placement": {"surrealdb": {"role": "storage_primary"}},
            "scan_scope": ["wrapper.sh"],
            "forbidden_patterns": [
                {
                    "id": "cortex_local_mcp_build",
                    "pattern": "cargo build -p cynic-mcp",
                    "severity": "critical",
                    "detail": "MCP builds must be lifecycle-dispatched."
                }
            ]
        });

        let findings = infra_consumption_contract_findings(
            temp.to_str().expect("temp path should be utf-8"),
            "axioms/infra-consumption-v1.json",
            &contract,
        );
        let _ = std::fs::remove_dir_all(&temp);

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str())
                == Some("infra_consumption_forbidden_cortex_local_mcp_build")
        }));
    }

    #[test]
    fn infra_consumption_contract_rejects_cortex_local_surrealdb_endpoint() {
        let temp = temp_fixture_dir("infra-consumption-surreal");
        let scanned = temp.join("Makefile");
        std::fs::write(
            &scanned,
            "backup:\n\tsurreal export --endpoint http://localhost:8000\n",
        )
        .expect("fixture file should be written");
        let contract = serde_json::json!({
            "schema": "cynic.infra_consumption.v1",
            "status": "normative",
            "capability_roles": ["storage_primary"],
            "required_ports": ["StoragePrimaryPort"],
            "native_lifecycle": {"forbidden_on_cortex": ["surreal --endpoint http://localhost:8000"]},
            "service_placement": {"surrealdb": {"role": "storage_primary"}},
            "scan_scope": ["Makefile"],
            "forbidden_patterns": [
                {
                    "id": "local_surrealdb_endpoint",
                    "pattern": "surreal (import|sql|export|is-ready) --endpoint http://localhost:8000",
                    "severity": "critical",
                    "detail": "SurrealDB must be consumed through storage_primary/Synapse or Kernel coordination API."
                }
            ]
        });

        let findings = infra_consumption_contract_findings(
            temp.to_str().expect("temp path should be utf-8"),
            "axioms/infra-consumption-v1.json",
            &contract,
        );
        let _ = std::fs::remove_dir_all(&temp);

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str())
                == Some("infra_consumption_forbidden_local_surrealdb_endpoint")
        }));
    }
}

