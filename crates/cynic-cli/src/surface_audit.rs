use crate::agent_exposure_audit::audit_agent_exposure;
use crate::surface_audit::checks::{audit_surface, duplicate_ids};
use crate::surface_audit::external_projection::audit_external_projection_files;
use crate::surface_registry::canonical_cli_surfaces;

mod checks;
mod external_projection;
mod inference_collector;
mod paths;
mod types;

pub(crate) use types::{ExternalProjectionFinding, SurfaceAuditEntry, SurfaceAuditReport};

pub(crate) fn audit_canonical_surfaces() -> SurfaceAuditReport {
    let surfaces = canonical_cli_surfaces();
    let ids = duplicate_ids(&surfaces);
    let entries = surfaces
        .iter()
        .map(|surface| audit_surface(surface, &ids))
        .collect::<Vec<_>>();
    let external_projection_findings = audit_external_projection_files();
    let agent_exposure = audit_agent_exposure();
    let ok_count = entries.iter().filter(|entry| entry.status == "ok").count();
    let blocked_surface_count = entries.len().saturating_sub(ok_count);
    let blocked_projection_count = external_projection_findings
        .iter()
        .filter(|finding| finding.status != "ok")
        .count();
    let blocked_agent_exposure = if agent_exposure.status == "ok" {
        0
    } else {
        agent_exposure.blocked_count
    };
    let blocked_count = blocked_surface_count + blocked_projection_count + blocked_agent_exposure;
    SurfaceAuditReport {
        schema: "cynic.cli.surface_audit.v1",
        status: if blocked_count == 0 { "ok" } else { "blocked" },
        rule: "canonical CLI surfaces must be backed by real adapter modules, declared owners, mutation policies, evidence, anti-responsibilities, and a faithful portable agent-exposure projection",
        surface_count: entries.len(),
        ok_count,
        blocked_count,
        surfaces: entries,
        external_projection_findings,
        agent_exposure,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_surfaces_hold_explicit_mcp_containment() {
        // Explicit temporary containment (AGENTS.md repair law): every surface
        // is ok, but agent_exposure stays blocked while declared tools lack
        // in-repo `#[tool]` counterparts. Gradient, not binary: each tool
        // implemented in cynic-mcp shrinks the phantom count by exactly one.
        // Removal condition: zero phantoms — restore the all-ok assertion.
        let report = audit_canonical_surfaces();
        assert!(report.surfaces.iter().all(|entry| entry.status == "ok"));
        assert!(report
            .external_projection_findings
            .iter()
            .all(|finding| finding.status == "ok"));
        assert_eq!(report.agent_exposure.status, "blocked");
        assert!(report.agent_exposure.source_tool_count >= 3);
        assert!(report.agent_exposure.findings.iter().all(|finding| {
            finding.kind == "exposure_tool_not_in_source" && finding.severity == "critical"
        }));
        assert_eq!(report.status, "blocked");
        assert_eq!(report.blocked_count, report.agent_exposure.blocked_count);
    }
}
