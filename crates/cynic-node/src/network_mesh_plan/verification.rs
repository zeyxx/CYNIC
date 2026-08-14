use std::collections::BTreeMap;

use super::*;

fn empty_policy() -> NetworkPolicy {
    NetworkPolicy {
        canonical_service_plane: None,
        planes: BTreeMap::new(),
        reservations: Vec::new(),
        mesh: None,
    }
}

#[kani::proof]
fn proof_address_plane_prefers_declared_service_plane() {
    let service_plane = "labmesh".to_string();
    let mut policy = empty_policy();
    policy.planes.insert(
        service_plane.clone(),
        NetworkPlane {
            cidr: Some("10.30.0.0/24".to_string()),
            dns_zone: None,
            canonical: Some(false),
        },
    );
    policy.planes.insert(
        "fallback".to_string(),
        NetworkPlane {
            cidr: Some("10.31.0.0/24".to_string()),
            dns_zone: None,
            canonical: Some(true),
        },
    );

    assert_eq!(address_plane(Some(&policy), &service_plane), service_plane,);
}

#[kani::proof]
fn proof_address_plane_falls_back_to_canonical_plane() {
    let mut policy = empty_policy();
    policy.planes.insert(
        "labmesh".to_string(),
        NetworkPlane {
            cidr: Some("10.30.0.0/24".to_string()),
            dns_zone: Some("labmesh.example".to_string()),
            canonical: Some(true),
        },
    );

    assert_eq!(address_plane(Some(&policy), "missing"), "labmesh");
}

#[kani::proof]
fn proof_address_plane_defaults_to_synapse_without_match() {
    let policy = empty_policy();

    assert_eq!(address_plane(Some(&policy), "missing"), "synapse");
    assert_eq!(address_plane(None, "missing"), "missing");
}

#[kani::proof]
fn proof_default_dns_names_follow_zone_presence() {
    let has_zone: bool = kani::any();
    let plane = NetworkPlane {
        cidr: None,
        dns_zone: has_zone.then(|| "labmesh.example".to_string()),
        canonical: None,
    };

    let names = default_dns_names("node-a", Some(&plane));
    if has_zone {
        assert_eq!(names, vec!["node-a.labmesh.example".to_string()]);
    } else {
        assert!(names.is_empty());
    }
}
