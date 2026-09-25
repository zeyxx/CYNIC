use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct NetworkSurfaceObservation {
    pub(crate) interface: String,
    pub(crate) state: String,
    pub(crate) addresses: Vec<String>,
    pub(crate) routes: Vec<String>,
    pub(crate) default_route: bool,
    pub(crate) link_up: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct RouteSurfaceClassification {
    pub(crate) destination: String,
    pub(crate) candidates: Vec<String>,
    pub(crate) preferred: Option<String>,
    pub(crate) status: String,
}

pub(crate) fn inventory_network_surfaces(
    interfaces_stdout: &str,
    routes_stdout: &str,
) -> Vec<NetworkSurfaceObservation> {
    let mut surfaces = interfaces_stdout
        .lines()
        .filter_map(parse_interface_line)
        .collect::<Vec<_>>();
    for line in routes_stdout.lines() {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        let Some(dev_index) = fields.iter().position(|field| *field == "dev") else {
            continue;
        };
        let Some(interface) = fields.get(dev_index + 1) else {
            continue;
        };
        let destination = fields.first().copied().unwrap_or_default();
        let default_route = destination == "default";
        let route = line.trim().to_string();
<<<<<<< HEAD
        if let Some(surface) = surfaces
            .iter_mut()
            .find(|surface| surface.interface == *interface)
        {
=======
        if let Some(surface) = surfaces.iter_mut().find(|surface| surface.interface == *interface) {
>>>>>>> github/main
            surface.routes.push(route);
            surface.default_route |= default_route;
        } else {
            surfaces.push(NetworkSurfaceObservation {
                interface: interface.to_string(),
                state: "unknown".to_string(),
                addresses: Vec::new(),
                routes: vec![route],
                default_route,
                link_up: false,
            });
        }
    }
    surfaces
}

pub(crate) fn classify_route_surfaces(routes_stdout: &str) -> Vec<RouteSurfaceClassification> {
    let mut grouped = BTreeMap::<String, Vec<(u32, String)>>::new();
    for line in routes_stdout.lines() {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        let Some(destination) = fields.first() else {
            continue;
        };
        if !fields.iter().any(|field| *field == "dev") {
            continue;
        }
        let metric = fields
            .iter()
            .position(|field| *field == "metric")
            .and_then(|index| fields.get(index + 1))
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(u32::MAX);
        grouped
            .entry((*destination).to_string())
            .or_default()
            .push((metric, line.trim().to_string()));
    }
    grouped
        .into_iter()
        .map(|(destination, mut candidates)| {
            candidates.sort_by_key(|(metric, _)| *metric);
            let rendered = candidates
                .iter()
                .map(|(_, route)| route.clone())
                .collect::<Vec<_>>();
            let status = if rendered.len() <= 1 {
                "clear"
            } else if candidates
                .first()
                .zip(candidates.get(1))
                .is_some_and(|(left, right)| left.0 == right.0)
            {
                "ambiguous"
            } else {
                "multiple_paths"
            };
            RouteSurfaceClassification {
                destination,
                preferred: rendered.first().cloned(),
                status: status.to_string(),
                candidates: rendered,
            }
        })
        .collect()
}

fn parse_interface_line(line: &str) -> Option<NetworkSurfaceObservation> {
    let fields = line.split_whitespace().collect::<Vec<_>>();
    let interface = fields.first()?.to_string();
    let state = fields.get(1)?.to_string();
    let addresses = fields
        .iter()
        .skip(2)
        .filter(|field| field.contains('/') && !field.starts_with("@"))
        .map(|field| field.to_string())
        .collect();
    Some(NetworkSurfaceObservation {
        interface,
        state: state.clone(),
        addresses,
        routes: Vec::new(),
        default_route: false,
        link_up: state == "UP" || state == "UNKNOWN",
    })
}

#[cfg(test)]
mod tests {
    use super::{classify_route_surfaces, inventory_network_surfaces};

    #[test]
    fn inventories_all_interfaces_and_routes_without_collapsing_surfaces() {
        let surfaces = inventory_network_surfaces(
            "lo UNKNOWN 127.0.0.1/8\nenp42s0 UP 192.168.100.2/24\nwlp41s0 UP 192.168.0.12/24\n",
            "default via 192.168.0.254 dev wlp41s0 metric 600\n10.10.10.0/24 via 192.168.100.1 dev enp42s0 metric 100\n",
        );
        assert_eq!(surfaces.len(), 3);
        let wired = surfaces
            .iter()
            .find(|surface| surface.interface == "enp42s0")
            .expect("wired surface");
        assert_eq!(wired.addresses, vec!["192.168.100.2/24"]);
        assert_eq!(wired.routes.len(), 1);
        assert!(!wired.default_route);
        let wifi = surfaces
            .iter()
            .find(|surface| surface.interface == "wlp41s0")
            .expect("wifi surface");
        assert!(wifi.default_route);
    }

    #[test]
    fn classifies_duplicate_routes_by_metric_without_hiding_the_secondary() {
        let routes = classify_route_surfaces(
            "10.10.10.0/24 via 192.168.100.1 dev enp42s0 metric 100\n10.10.10.0/24 via 192.168.0.37 dev wlp41s0 metric 600\n",
        );
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].status, "multiple_paths");
        assert_eq!(routes[0].candidates.len(), 2);
<<<<<<< HEAD
        assert!(
            routes[0]
                .preferred
                .as_deref()
                .is_some_and(|route| route.contains("enp42s0"))
        );
=======
        assert!(routes[0]
            .preferred
            .as_deref()
            .is_some_and(|route| route.contains("enp42s0")));
>>>>>>> github/main
    }
}
