use serde::Serialize;

pub const RUNTIME_VALIDATION_SCHEMA: &str = "cynic.runtime_validation.v1";

mod campaign;

pub use campaign::{
    DEVELOPMENT_CAMPAIGN_EVENT_SCHEMA, DEVELOPMENT_CAMPAIGN_PROJECTION_SCHEMA,
    DEVELOPMENT_CAMPAIGN_SCHEMA, DevelopmentCampaign, DevelopmentCampaignEvent,
    DevelopmentCampaignEventKind, DevelopmentCampaignMilestone, DevelopmentCampaignProjection,
    DevelopmentMilestoneProjection, DevelopmentMilestoneStatus, reconcile_development_campaign,
};

                anti_responsibilities: vec![
                    "prove real fleet health",
                    "replace single-node E2E",
                    "depend on machine-local hostnames",
                ],
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn development_surfaces_have_unique_ids_and_owners() {
        let manifest = development_surface_manifest();
        let mut ids = BTreeSet::new();
        for surface in &manifest.surfaces {
            assert!(ids.insert(surface.id), "duplicate surface {}", surface.id);
            assert!(!surface.owner.is_empty(), "surface owner required");
            assert!(
                !surface.canonical_commands.is_empty(),
                "surface commands required"
            );
        }
    }

    #[test]
    fn development_surfaces_keep_cli_as_adapter() {
        let manifest = development_surface_manifest();
        let cli = manifest
            .surfaces
            .iter()
            .find(|surface| surface.owner == "cynic-cli")
            .expect("cli surface exists");
        assert!(cli.anti_responsibilities.contains(&"own planner rules"));
    }
}
