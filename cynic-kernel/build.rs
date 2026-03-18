fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Inject git describe into the binary — traceable builds without manual versioning.
    // Fallback to Cargo.toml version if git is unavailable (container builds).
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/refs/tags");
    let git_version = std::process::Command::new("git")
        .args(["describe", "--tags", "--always", "--dirty"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());
    println!("cargo:rustc-env=CYNIC_VERSION={}", git_version);

    #[cfg(feature = "grpc")]
    {
        println!("cargo:rerun-if-changed=../protos/cynic.proto");

        let protoc_path = protoc_bin_vendored::protoc_bin_path().expect(
            "FATAL: Could not locate bundled protoc binary."
        );
        unsafe { std::env::set_var("PROTOC", protoc_path); }

        tonic_build::configure()
            .build_server(true)
            .build_client(true)
            .compile_protos(&["../protos/cynic.proto"], &["../protos"])?;
    }
    Ok(())
}
