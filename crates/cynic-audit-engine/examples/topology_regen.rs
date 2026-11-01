//! Regenerate `infra/crate-topology.json` from the live workspace.
//!
//! Usage: cargo run -p cynic-audit-engine --example topology_regen -- <root>

use std::path::PathBuf;
use cynic_audit_engine::topology_generate;

fn main() {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    let rendered = topology_generate::render_text(&root).expect("render topology SSOT");
    let path = root.join(topology_generate::TOPOLOGY_DOC);
    std::fs::write(&path, rendered).expect("write topology SSOT");
    println!("Topology successfully regenerated at {}", path.display());
}
