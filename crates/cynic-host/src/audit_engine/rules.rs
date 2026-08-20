use cynic_protocol::{
    AuditCategory, AuditRuleSpec, AuditSeverity, AuditSpan, AuditViolation,
};

use super::scanner::WorkspaceSourceIndex;

pub trait AuditRule {
    fn spec(&self) -> AuditRuleSpec;
    fn evaluate(&self, index: &WorkspaceSourceIndex) -> Vec<AuditViolation>;
}

pub struct PathSanitationRule;

impl AuditRule for PathSanitationRule {
    fn spec(&self) -> AuditRuleSpec {
        AuditRuleSpec {
            id: "SEC-001".to_string(),
            name: "path_sanitation".to_string(),
            description: "Committed source files and projections must not embed machine-local absolute paths (/home/, Users).".to_string(),
            category: AuditCategory::PathSanitation,
            severity: AuditSeverity::Critical,
            automated_remediation_available: false,
        }
    }

    fn evaluate(&self, index: &WorkspaceSourceIndex) -> Vec<AuditViolation> {
        let mut violations = Vec::new();

        let all_files = index
            .rust_files
            .iter()
            .chain(index.json_files.iter())
            .chain(index.manifests.iter());

        for file in all_files {
            // Ignore audit rules themselves and test data
            if file.relative_path.contains("audit_engine")
                || file.relative_path.contains("agent_exposure_audit")
                || file.relative_path.contains("test")
            {
                continue;
            }

            // Track `#[cfg(test)]` scope by brace depth. Filtering on the file
            // path alone cannot see inline test modules, which is where most
            // machine-local paths in this workspace legitimately live: they are
            // fixtures asserting how a path is classified, not paths the runtime
            // ever uses. Counting them as critical made the rule unactionable.
            let mut cfg_test_depth: Option<i32> = None;
            let mut depth: i32 = 0;
            let mut pending_cfg_test = false;

            for (idx, line) in file.content.lines().enumerate() {
                if line.contains("cfg(test)") {
                    pending_cfg_test = true;
                }

                let opens = line.matches('{').count() as i32;
                let closes = line.matches('}').count() as i32;

                if pending_cfg_test && opens > 0 {
                    cfg_test_depth = Some(depth);
                    pending_cfg_test = false;
                }

                depth += opens - closes;

                if let Some(entry_depth) = cfg_test_depth {
                    if depth <= entry_depth {
                        cfg_test_depth = None;
                    }
                }

                let in_test_scope = cfg_test_depth.is_some();

                // A line that *inspects* a path prefix is not a leak: several
                // surfaces legitimately detect operator-home paths in order to
                // reject or reclassify them. Only a path embedded as a value is
                // a violation.
                let inspects_rather_than_embeds = line.contains("starts_with(")
                    || line.contains("contains(")
                    || line.contains("for prefix in");

                if !in_test_scope
                    && !inspects_rather_than_embeds
                    && (line.contains("/home/")
                        || line.contains("/Users/")
                        || line.contains("C:\\Users\\")
                        || line.contains("C:/Users/"))
                {
                    violations.push(AuditViolation {
                        code: "SEC-001-ABSOLUTE-PATH".to_string(),
                        message: format!(
                            "Machine-local path pattern detected in {}",
                            file.relative_path
                        ),
                        severity: AuditSeverity::Critical,
                        category: AuditCategory::PathSanitation,
                        span: Some(AuditSpan {
                            file_path: file.relative_path.clone(),
                            line_start: idx + 1,
                            line_end: idx + 1,
                            snippet: Some(line.trim().to_string()),
                        }),
                        remediation: Some(
                            "Inject runtime path dynamically via layout or deployment context."
                                .to_string(),
                        ),
                    });
                }
            }
        }

        violations
    }
}

pub struct HotspotThresholdRule;

impl AuditRule for HotspotThresholdRule {
    fn spec(&self) -> AuditRuleSpec {
        AuditRuleSpec {
            id: "ARCH-001".to_string(),
            name: "hotspot_threshold".to_string(),
            description: "Domain and surface files must not exceed 400 lines without modular extraction.".to_string(),
            category: AuditCategory::HotspotThreshold,
            severity: AuditSeverity::Debt,
            automated_remediation_available: true,
        }
    }

    fn evaluate(&self, index: &WorkspaceSourceIndex) -> Vec<AuditViolation> {
        let mut violations = Vec::new();

        for file in &index.rust_files {
            // Ignore pure test files and generated tables
            if file.relative_path.contains("/tests/")
                || file.relative_path.contains("surface_registry/catalog.rs")
            {
                continue;
            }

            if file.line_count > 400 {
                violations.push(AuditViolation {
                    code: "ARCH-001-HOTSPOT".to_string(),
                    message: format!(
                        "File {} has {} lines (exceeds 400 line hard threshold)",
                        file.relative_path, file.line_count
                    ),
                    severity: AuditSeverity::Debt,
                    category: AuditCategory::HotspotThreshold,
                    span: Some(AuditSpan {
                        file_path: file.relative_path.clone(),
                        line_start: 1,
                        line_end: file.line_count,
                        snippet: None,
                    }),
                    remediation: Some(
                        "Split into types, policy, ports, adapters, and thin facades.".to_string(),
                    ),
                });
            }
        }

        violations
    }
}

pub struct LayerBoundaryRule;

impl AuditRule for LayerBoundaryRule {
    fn spec(&self) -> AuditRuleSpec {
        AuditRuleSpec {
            id: "ARCH-002".to_string(),
            name: "layer_boundary".to_string(),
            description: "Protocol contracts must remain pure and not import runtime I/O adapters.".to_string(),
            category: AuditCategory::Boundary,
            severity: AuditSeverity::Critical,
            automated_remediation_available: false,
        }
    }

    fn evaluate(&self, index: &WorkspaceSourceIndex) -> Vec<AuditViolation> {
        let mut violations = Vec::new();

        for file in &index.rust_files {
            if !file.relative_path.starts_with("crates/cynic-protocol/src/") {
                continue;
            }

            for (idx, line) in file.content.lines().enumerate() {
                let trimmed = line.trim();
                if trimmed.starts_with("//") {
                    continue;
                }

                if line.contains("std::process::Command")
                    || line.contains("tokio::process::Command")
                    || line.contains("reqwest::")
                    || line.contains("clap::Parser")
                {
                    violations.push(AuditViolation {
                        code: "ARCH-002-FORBIDDEN-ADAPTER".to_string(),
                        message: format!(
                            "Protocol file {} imports forbidden I/O adapter: {}",
                            file.relative_path, trimmed
                        ),
                        severity: AuditSeverity::Critical,
                        category: AuditCategory::Boundary,
                        span: Some(AuditSpan {
                            file_path: file.relative_path.clone(),
                            line_start: idx + 1,
                            line_end: idx + 1,
                            snippet: Some(trimmed.to_string()),
                        }),
                        remediation: Some(
                            "Move adapter implementation to cynic-host or cynic-node.".to_string(),
                        ),
                    });
                }
            }
        }

        violations
    }
}

pub struct KernelBoundaryRule;

impl AuditRule for KernelBoundaryRule {
    fn spec(&self) -> AuditRuleSpec {
        AuditRuleSpec {
            id: "KERNEL-001".to_string(),
            name: "kernel_boundary".to_string(),
            description: "Kernel should retain state bus and consensus, extracting inference and host lifecycle.".to_string(),
            category: AuditCategory::Boundary,
            severity: AuditSeverity::Debt,
            automated_remediation_available: true,
        }
    }

    fn evaluate(&self, index: &WorkspaceSourceIndex) -> Vec<AuditViolation> {
        let mut violations = Vec::new();

        for file in &index.rust_files {
            if !file.relative_path.starts_with("crates/cynic-kernel/src/") {
                continue;
            }

            if file.relative_path.contains("inference")
                || file.relative_path.contains("auto_embed")
                || file.relative_path.contains("backends/")
            {
                violations.push(AuditViolation {
                    code: "KERNEL-001-INFERENCE-STACK".to_string(),
                    message: format!(
                        "Inference stack file {} belongs in cynic-inference, not cynic-kernel",
                        file.relative_path
                    ),
                    severity: AuditSeverity::Debt,
                    category: AuditCategory::Boundary,
                    span: Some(AuditSpan {
                        file_path: file.relative_path.clone(),
                        line_start: 1,
                        line_end: file.line_count,
                        snippet: None,
                    }),
                    remediation: Some(
                        "Extract to crates/cynic-inference/src/".to_string(),
                    ),
                });
            }
        }

        violations
    }
}

pub struct NodeHostBoundaryRule;

impl AuditRule for NodeHostBoundaryRule {
    fn spec(&self) -> AuditRuleSpec {
        AuditRuleSpec {
            id: "NODE-001".to_string(),
            name: "node_host_boundary".to_string(),
            description: "Node runtime must not manage host lifecycle directly without ports.".to_string(),
            category: AuditCategory::Boundary,
            severity: AuditSeverity::Warning,
            automated_remediation_available: false,
        }
    }

    fn evaluate(&self, index: &WorkspaceSourceIndex) -> Vec<AuditViolation> {
        let mut violations = Vec::new();

        for file in &index.rust_files {
            if !file.relative_path.starts_with("crates/cynic-node/src/") {
                continue;
            }

            for (idx, line) in file.content.lines().enumerate() {
                let trimmed = line.trim();
                if trimmed.starts_with("//") {
                    continue;
                }

                if line.contains("cynic_host::host_exec_cli")
                    || line.contains("cynic_host::bootstrap_host_plan")
                {
                    violations.push(AuditViolation {
                        code: "NODE-001-DIRECT-HOST-EXEC".to_string(),
                        message: format!(
                            "Node file {} directly couples to host exec: {}",
                            file.relative_path, trimmed
                        ),
                        severity: AuditSeverity::Warning,
                        category: AuditCategory::Boundary,
                        span: Some(AuditSpan {
                            file_path: file.relative_path.clone(),
                            line_start: idx + 1,
                            line_end: idx + 1,
                            snippet: Some(trimmed.to_string()),
                        }),
                        remediation: Some(
                            "Use an injected port trait instead of direct cynic-host coupling.".to_string(),
                        ),
                    });
                }
            }
        }

        violations
    }
}

pub struct OwnershipRule;

impl AuditRule for OwnershipRule {
    fn spec(&self) -> AuditRuleSpec {
        AuditRuleSpec {
            id: "OWN-001".to_string(),
            name: "principal_ownership".to_string(),
            description: "Installed runtime state must belong to cynic principal, not operator home.".to_string(),
            category: AuditCategory::MutationContract,
            severity: AuditSeverity::Critical,
            automated_remediation_available: false,
        }
    }

    fn evaluate(&self, index: &WorkspaceSourceIndex) -> Vec<AuditViolation> {
        let mut violations = Vec::new();

        for file in &index.rust_files {
            if file.relative_path.contains("test")
                || file.relative_path.contains("audit_engine")
                || file.relative_path.contains("ownership_audit")
            {
                continue;
            }

            for (idx, line) in file.content.lines().enumerate() {
                let trimmed = line.trim();
                if trimmed.starts_with("//") {
                    continue;
                }

                if (line.contains("/var/lib/cynic") || line.contains("/etc/cynic"))
                    && line.contains("std::fs::write")
                {
                    violations.push(AuditViolation {
                        code: "OWN-001-UNGUARDED-WRITE".to_string(),
                        message: format!(
                            "Unguarded write to runtime principal directory in {}: {}",
                            file.relative_path, trimmed
                        ),
                        severity: AuditSeverity::Critical,
                        category: AuditCategory::MutationContract,
                        span: Some(AuditSpan {
                            file_path: file.relative_path.clone(),
                            line_start: idx + 1,
                            line_end: idx + 1,
                            snippet: Some(trimmed.to_string()),
                        }),
                        remediation: Some(
                            "Pass mutation through cynic-host principal prepare or activation request.".to_string(),
                        ),
                    });
                }
            }
        }

        violations
    }
}
