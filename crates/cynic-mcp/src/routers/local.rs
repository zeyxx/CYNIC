// Thin assembly surface. Each bounded family owns its generated router.
include!("local/fleet.rs");
include!("local/operator_transport.rs");
include!("local/agent_surfaces.rs");
include!("local/infra_registry.rs");
include!("local/network.rs");
include!("local/fleet_execution.rs");
include!("local/services.rs");
include!("local/storage.rs");
include!("local/legacy_proxmox.rs");
include!("local/lifecycle_inference.rs");
