use std::fs;
use std::path::{Path, PathBuf};

fn sandbox_crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn sandbox_domain_is_pure_zero_io() {
    let domain_files = [
        "src/bwrap.rs",
        "src/cargo.rs",
        "src/egress_proxy.rs",
        "src/netns.rs",
        "src/package.rs",
        "src/rapl.rs",
    ];

    for rel_path in &domain_files {
        let path = sandbox_crate_root().join(rel_path);
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));

        let forbidden = ["tokio::spawn", "reqwest::", "std::net::TcpStream"];
        for pattern in &forbidden {
            assert!(
                !content.contains(pattern),
                "Architecture violation in {rel_path}: domain module contains forbidden pattern '{pattern}'"
            );
        }
    }
}
