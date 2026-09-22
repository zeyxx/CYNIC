use std::fs;
use std::path::{Path, PathBuf};

fn inference_crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn inference_domain_is_pure_zero_io() {
    let cargo_toml =
        fs::read_to_string(inference_crate_root().join("Cargo.toml")).expect("read Cargo.toml");

    let forbidden_crates = ["cynic-host", "cynic-kernel", "cynic-node", "cynic-cli"];
    for forbidden in &forbidden_crates {
        assert!(
            !cargo_toml.contains(forbidden),
            "Architecture violation: cynic-inference must never depend on orchestrator or host crates ({forbidden})"
        );
    }
}

/// Domain layer MUST NOT depend on std::fs, tokio, reqwest, axum,
/// futures_util, or crate::adapters::.
#[test]
fn domain_is_pure_no_io_or_async_deps() {
    let domain_dir = inference_crate_root().join("src/domain");
    let mut violations = Vec::new();

    for entry in fs::read_dir(&domain_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        if path.file_name().unwrap() == "mod.rs" {
            continue;
        }

        let content = fs::read_to_string(&path).unwrap();
        for (lineno, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if !trimmed.starts_with("use ") {
                continue;
            }
            // Allow std::collections (pure data structures)
            if trimmed.starts_with("use std::collections::") {
                continue;
            }
            if trimmed.contains("use std::")
                || trimmed.contains("use tokio::")
                || trimmed.contains("use reqwest::")
                || trimmed.contains("use axum::")
                || trimmed.contains("use futures_util::")
                || trimmed.contains("use crate::adapters::")
                || trimmed.contains("use crate::infra::")
            {
                violations.push(format!(
                    "{}:{}: {}",
                    path.file_name().unwrap().to_str().unwrap(),
                    lineno + 1,
                    trimmed
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "domain/ purity violations:\n{}",
        violations.join("\n")
    );
}

/// Ports layer MUST contain trait definitions. Associated Error/DTO types are
/// legitimate (e.g. `pub enum FooError {}`, `pub struct FooResult {}`).
/// Forbidden: concrete `impl Foo for Bar` blocks (adapters own implementations).
#[test]
fn ports_define_traits_not_impls() {
    let ports_dir = inference_crate_root().join("src/ports");
    let mut violations = Vec::new();

    for entry in fs::read_dir(&ports_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        if path.file_name().unwrap() == "mod.rs" {
            continue;
        }

        let content = fs::read_to_string(&path).unwrap();
        let has_trait = content.contains("pub trait ") || content.contains("trait ");
        let has_impl_block = content.contains("impl ");

        if !has_trait {
            violations.push(format!(
                "{}: file has no trait definition",
                path.file_name().unwrap().to_str().unwrap()
            ));
        }
        if has_impl_block && !content.contains("pub trait ") {
            violations.push(format!(
                "{}: ports must not contain impl blocks (adapters own impls)",
                path.file_name().unwrap().to_str().unwrap()
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ports/ architecture violations:\n{}",
        violations.join("\n")
    );
}

/// Workspace DAG: domain MUST NOT depend on ports (ports depend on domain).
#[test]
fn domain_does_not_import_ports() {
    let domain_dir = inference_crate_root().join("src/domain");
    let mut violations = Vec::new();

    for entry in fs::read_dir(&domain_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }

        let content = fs::read_to_string(&path).unwrap();
        for (lineno, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.contains("use crate::ports::") || trimmed.contains("use super::ports::") {
                violations.push(format!(
                    "{}:{}: {}",
                    path.file_name().unwrap().to_str().unwrap(),
                    lineno + 1,
                    trimmed
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "domain must not import ports:\n{}",
        violations.join("\n")
    );
}

/// EngineCapabilities domain type must have pure variants.
#[test]
fn engine_caps_is_pure_enum() {
    let path = inference_crate_root().join("src/domain/engine_caps.rs");
    let content = fs::read_to_string(&path).unwrap();

    assert!(content.contains("pub struct EngineCapabilities"));
    assert!(content.contains("pub enum ArchStatus"));
    assert!(content.contains("pub enum Observation"));
    assert!(content.contains("pub enum PreflightDecision"));

    assert!(!content.contains("tokio"));
    assert!(!content.contains("async fn"));
    assert!(!content.contains("std::fs"));
    assert!(!content.contains("std::io"));
}
