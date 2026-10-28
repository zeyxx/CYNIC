//! Pure drift detection for registry-declared host families.
//!
//! A node may declare a `host_family` the runtime resolver does not know.
//! The resolver answers `None` for an unrecognised family, and the caller
//! then falls back to a wildcard arm. That fallback is silent: the node
//! still resolves, to a profile that was never declared.
//!
//! This predicate separates four outcomes that the resolver conflates:
//!
//! * `Absent` — the node declares no family at all.
//! * `Declared` — the family is declared and the resolver recognises it.
//! * `Unrecognised` — the family is declared and the resolver does not.
//! * `Empty` — a family is declared but blank, so it says nothing.
//!
//! `Unrecognised` is the condition that must never be silent: the node is
//! running under a profile nobody chose.

use std::fmt;

/// Families the runtime resolver accepts, keyed by the declaration site.
///
/// Kept in sync with `cynic_host::registry::host_profile_for_family`, which
/// is the authority. This table is a duplicate on purpose: the predicate
/// must stay pure and dependency-free, and a divergence between the two is
/// itself a drift signal rather than a compile error.
pub const RESOLVED_FAMILIES: &[&str] = &[
    "debian-systemd",
    "linux.debian.systemd",
    "ubuntu-systemd",
    "linux.ubuntu.systemd",
    "alpine-openrc",
    "linux.alpine.openrc",
    "proxmox",
    "proxmox-hypervisor",
    "linux.proxmox.systemd",
    "windows",
    "windows-service",
    "windows.generic",
];

/// Why a node declares a family, and whether the resolver honours it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostFamilyDrift {
    /// No family declared. Resolution then depends on role alone.
    Absent,
    /// Declared and recognised. This node resolves as intended.
    Declared { family: String },
    /// Declared but not in [`RESOLVED_FAMILIES`].
    ///
    /// The resolver returns `None` here and the caller substitutes its
    /// default profile. The node is therefore running under a profile that
    /// nobody declared for it.
    Unrecognised { family: String },
    /// The declaration is present but blank, so it carries no information.
    Empty,
}

impl HostFamilyDrift {
    /// Classify one node's declared family.
    pub fn classify(declared: Option<&str>) -> Self {
        let raw = match declared {
            None => return Self::Absent,
            Some(value) => value.trim(),
        };
        if raw.is_empty() {
            return Self::Empty;
        }
        if RESOLVED_FAMILIES.contains(&raw) {
            Self::Declared {
                family: raw.to_string(),
            }
        } else {
            Self::Unrecognised {
                family: raw.to_string(),
            }
        }
    }

    /// True when the node will resolve under a profile it did not declare.
    pub fn is_silent_fallback(&self) -> bool {
        matches!(self, Self::Unrecognised { .. })
    }

    /// True when the resolver honours the declaration exactly.
    pub fn is_resolved(&self) -> bool {
        matches!(self, Self::Declared { .. })
    }

    /// Stable machine-readable reason code.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Absent => "absent",
            Self::Declared { .. } => "declared",
            Self::Unrecognised { .. } => "unrecognised_family",
            Self::Empty => "empty_family",
        }
    }

    /// The declared family, when one exists.
    pub fn family(&self) -> Option<&str> {
        match self {
            Self::Declared { family } | Self::Unrecognised { family } => Some(family.as_str()),
            _ => None,
        }
    }
}

impl fmt::Display for HostFamilyDrift {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Absent => write!(f, "no host family declared"),
            Self::Declared { family } => write!(f, "host family {family} is resolved"),
            Self::Unrecognised { family } => write!(
                f,
                "host family {family} is not in the resolver table, so the node resolves under a default profile"
            ),
            Self::Empty => write!(f, "host family is declared but blank"),
        }
    }
}

/// A node's family drift, paired with the identifier it was read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostFamilyObservation {
    pub node_id: String,
    pub drift: HostFamilyDrift,
}

impl HostFamilyObservation {
    pub fn new(node_id: impl Into<String>, declared: Option<&str>) -> Self {
        Self {
            node_id: node_id.into(),
            drift: HostFamilyDrift::classify(declared),
        }
    }
}

/// Every node whose family the resolver will not honour.
pub fn silent_fallbacks<'a, I>(observations: I) -> Vec<&'a HostFamilyObservation>
where
    I: IntoIterator<Item = &'a HostFamilyObservation>,
{
    observations
        .into_iter()
        .filter(|observation| observation.drift.is_silent_fallback())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_family_is_not_a_fallback() {
        let drift = HostFamilyDrift::classify(None);
        assert_eq!(drift, HostFamilyDrift::Absent);
        assert!(!drift.is_silent_fallback());
        assert_eq!(drift.code(), "absent");
    }

    #[test]
    fn blank_family_is_its_own_state() {
        let drift = HostFamilyDrift::classify(Some("   "));
        assert_eq!(drift, HostFamilyDrift::Empty);
        assert!(!drift.is_silent_fallback());
        assert_eq!(drift.code(), "empty_family");
    }

    #[test]
    fn known_linux_families_resolve() {
        for family in [
            "linux.debian.systemd",
            "linux.ubuntu.systemd",
            "linux.proxmox.systemd",
            "linux.alpine.openrc",
        ] {
            let drift = HostFamilyDrift::classify(Some(family));
            assert!(drift.is_resolved(), "{family} should resolve");
            assert_eq!(drift.code(), "declared");
        }
    }

    #[test]
    fn known_windows_families_resolve() {
        for family in ["windows", "windows-service", "windows.generic"] {
            let drift = HostFamilyDrift::classify(Some(family));
            assert!(drift.is_resolved(), "{family} should resolve");
        }
    }

    #[test]
    fn unknown_family_is_reported_not_swallowed() {
        // This is the exact value that was written to the registry by mistake
        // and that resolved silently to a Linux network node profile.
        let drift = HostFamilyDrift::classify(Some("windows.windows11"));
        assert_eq!(drift.code(), "unrecognised_family");
        assert!(drift.is_silent_fallback());
        assert!(!drift.is_resolved());
        assert_eq!(drift.family(), Some("windows.windows11"));
    }

    #[test]
    fn whitespace_around_a_known_family_is_tolerated() {
        let drift = HostFamilyDrift::classify(Some("  linux.ubuntu.systemd \n"));
        assert!(drift.is_resolved(), "trimming should not break resolution");
    }

    #[test]
    fn whitespace_around_an_unknown_family_still_drifts() {
        let drift = HostFamilyDrift::classify(Some(" windows.windows11 "));
        assert!(drift.is_silent_fallback());
        assert_eq!(drift.family(), Some("windows.windows11"));
    }

    #[test]
    fn display_names_the_consequence() {
        let drift = HostFamilyDrift::classify(Some("windows.windows11"));
        let text = drift.to_string();
        assert!(text.contains("windows.windows11"), "message lost the family");
        assert!(
            text.contains("default profile"),
            "message must state the silent substitution: {text}"
        );
    }

    #[test]
    fn silent_fallbacks_filters_and_preserves_order() {
        let observations = vec![
            HostFamilyObservation::new("a", Some("linux.ubuntu.systemd")),
            HostFamilyObservation::new("b", Some("windows.windows11")),
            HostFamilyObservation::new("c", None),
            HostFamilyObservation::new("d", Some("windows-service")),
            HostFamilyObservation::new("e", Some("linux.debian.windows")),
        ];
        let drifting = silent_fallbacks(&observations);
        let ids: Vec<&str> = drifting.iter().map(|o| o.node_id.as_str()).collect();
        assert_eq!(ids, vec!["b", "e"]);
    }

    #[test]
    fn registry_values_of_the_day_are_classified_correctly() {
        // Families actually present in infra/registry.json on 2026-10-07.
        let resolved = ["linux.debian.systemd", "linux.proxmox.systemd", "linux.ubuntu.systemd"];
        for family in resolved {
            assert!(HostFamilyDrift::classify(Some(family)).is_resolved());
        }
        // Nodes without a declared family must not be reported as drift.
        for node in ["cynic-host-cortex", "local-kernel", "forgejo"] {
            assert!(!HostFamilyObservation::new(node, None)
                .drift
                .is_silent_fallback());
        }
    }

    #[test]
    fn family_table_is_not_empty_and_covers_both_oses() {
        assert!(RESOLVED_FAMILIES.contains(&"windows-service"));
        assert!(RESOLVED_FAMILIES.contains(&"linux.ubuntu.systemd"));
        assert!(
            RESOLVED_FAMILIES.len() >= 12,
            "table shrank unexpectedly: {}",
            RESOLVED_FAMILIES.len()
        );
    }
}