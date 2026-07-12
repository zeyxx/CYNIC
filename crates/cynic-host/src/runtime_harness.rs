use serde::Serialize;

pub const RUNTIME_HARNESS_CONTEXT_SCHEMA: &str = "cynic.runtime_harness.context.v1";
pub const RUNTIME_HARNESS_CATALOG_SCHEMA: &str = "cynic.runtime_harness.catalog.v1";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RuntimeHarnessContext {
    pub schema: &'static str,
    pub kind: RuntimeHarnessKind,
    pub role: &'static str,
    pub authority: HarnessAuthority,
    pub isolation_boundary: &'static str,
    pub target_scope: &'static str,
    pub mutates_host_network: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RuntimeHarnessCatalogReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub selected_kind: Option<&'static str>,
    pub contexts: Vec<RuntimeHarnessContext>,
    pub progression: Vec<&'static str>,
    pub rules: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeHarnessKind {
    ClosedLocalLab,
    Conformance,
    Staging,
    Canary,
    Production,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HarnessAuthority {
    TestProjection,
    ConformanceGate,
    ReleaseCandidate,
    FleetCanary,
    RuntimeAuthority,
}

pub fn closed_local_lab_context() -> RuntimeHarnessContext {
    RuntimeHarnessContext {
        schema: RUNTIME_HARNESS_CONTEXT_SCHEMA,
        kind: RuntimeHarnessKind::ClosedLocalLab,
        role: "ephemeral proof environment for bootstrap, network, and runtime evidence",
        authority: HarnessAuthority::TestProjection,
        isolation_boundary: "local qemu/libvirt isolated network",
        target_scope: "single-host closed lab first, then N guest nodes",
        mutates_host_network: false,
    }
}

pub fn conformance_context() -> RuntimeHarnessContext {
    RuntimeHarnessContext {
        schema: RUNTIME_HARNESS_CONTEXT_SCHEMA,
        kind: RuntimeHarnessKind::Conformance,
        role: "portable node qualification suite for execution, bootstrap, security, and observation contracts",
        authority: HarnessAuthority::ConformanceGate,
        isolation_boundary: "bounded local or guest execution with no production authority",
        target_scope: "one candidate node first, then repeatable N-node qualification",
        mutates_host_network: false,
    }
}

pub fn staging_context() -> RuntimeHarnessContext {
    RuntimeHarnessContext {
        schema: RUNTIME_HARNESS_CONTEXT_SCHEMA,
        kind: RuntimeHarnessKind::Staging,
        role: "production-like release candidate validation before fleet rollout",
        authority: HarnessAuthority::ReleaseCandidate,
        isolation_boundary: "declared staging VM or container boundary",
        target_scope: "release candidate node or staging constellation",
        mutates_host_network: false,
    }
}

pub fn canary_context() -> RuntimeHarnessContext {
    RuntimeHarnessContext {
        schema: RUNTIME_HARNESS_CONTEXT_SCHEMA,
        kind: RuntimeHarnessKind::Canary,
        role: "bounded real-fleet rollout with observation and rollback requirements",
        authority: HarnessAuthority::FleetCanary,
        isolation_boundary: "declared canary node or fleet subset boundary",
        target_scope: "small declared subset of real runtime nodes",
        mutates_host_network: true,
    }
}

pub fn production_context() -> RuntimeHarnessContext {
    RuntimeHarnessContext {
        schema: RUNTIME_HARNESS_CONTEXT_SCHEMA,
        kind: RuntimeHarnessKind::Production,
        role: "declared runtime authority with persistent evidence and SLO tracking",
        authority: HarnessAuthority::RuntimeAuthority,
        isolation_boundary: "production fleet boundary declared by runtime authority",
        target_scope: "declared active organism runtime",
        mutates_host_network: true,
    }
}

pub fn runtime_harness_contexts() -> Vec<RuntimeHarnessContext> {
    vec![
        closed_local_lab_context(),
        conformance_context(),
        staging_context(),
        canary_context(),
        production_context(),
    ]
}

pub fn runtime_harness_catalog(
    selected_kind: Option<&str>,
) -> Result<RuntimeHarnessCatalogReport, String> {
    let selected_kind = selected_kind
        .map(|kind| {
            RuntimeHarnessKind::canonical_name(kind)
                .ok_or_else(|| format!("unknown runtime harness kind: {kind}"))
        })
        .transpose()?;
    let contexts = match selected_kind {
        Some(kind) => {
            let context = runtime_harness_contexts()
                .into_iter()
                .find(|context| context.kind.as_str() == kind)
                .ok_or_else(|| format!("runtime harness kind is not modeled: {kind}"))?;
            vec![context]
        }
        None => runtime_harness_contexts(),
    };

    Ok(RuntimeHarnessCatalogReport {
        schema: RUNTIME_HARNESS_CATALOG_SCHEMA,
        status: "ok",
        selected_kind,
        contexts,
        progression: vec![
            "closed_local_lab",
            "conformance",
            "staging",
            "canary",
            "production",
        ],
        rules: vec![
            "a harness is an execution membrane, not the capability itself",
            "lab must remain a closed local test projection",
            "conformance is the portable node qualification boundary",
            "canary and production may mutate real host/network state only after explicit authority",
        ],
    })
}

impl RuntimeHarnessKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ClosedLocalLab => "closed_local_lab",
            Self::Conformance => "conformance",
            Self::Staging => "staging",
            Self::Canary => "canary",
            Self::Production => "production",
        }
    }

    fn canonical_name(kind: &str) -> Option<&'static str> {
        match kind {
            "closed_local_lab" | "lab" => Some("closed_local_lab"),
            "conformance" => Some("conformance"),
            "staging" => Some("staging"),
            "canary" => Some("canary"),
            "production" => Some("production"),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        HarnessAuthority, RuntimeHarnessKind, closed_local_lab_context, conformance_context,
        runtime_harness_catalog,
    };

    #[test]
    fn closed_local_lab_is_not_runtime_authority() {
        let context = closed_local_lab_context();

        assert_eq!(context.schema, "cynic.runtime_harness.context.v1");
        assert_eq!(context.kind, RuntimeHarnessKind::ClosedLocalLab);
        assert_eq!(context.authority, HarnessAuthority::TestProjection);
        assert!(!context.mutates_host_network);
    }

    #[test]
    fn conformance_is_portable_and_not_production_authority() {
        let context = conformance_context();

        assert_eq!(context.kind, RuntimeHarnessKind::Conformance);
        assert_eq!(context.authority, HarnessAuthority::ConformanceGate);
        assert!(!context.mutates_host_network);
    }

    #[test]
    fn catalog_can_select_conformance() {
        let report = runtime_harness_catalog(Some("conformance")).expect("catalog");

        assert_eq!(report.schema, super::RUNTIME_HARNESS_CATALOG_SCHEMA);
        assert_eq!(report.status, "ok");
        assert_eq!(report.selected_kind, Some("conformance"));
        assert_eq!(report.contexts.len(), 1);
        assert_eq!(report.contexts[0].kind, RuntimeHarnessKind::Conformance);
    }
}
