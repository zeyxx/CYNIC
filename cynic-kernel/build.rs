fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=../protos/cynic.proto");
    
    // Use the bundled protoc binary — no system protoc required on any OS
    // This makes the build fully hermetic: works on Windows, WSL2, Linux, macOS
    let protoc_path = protoc_bin_vendored::protoc_bin_path().expect(
        "FATAL: Could not locate bundled protoc binary. Please report this as a CYNIC build system bug."
    );
    // SAFETY: set_var is safe during build scripts as they are single-threaded at startup
    unsafe { std::env::set_var("PROTOC", protoc_path); }
    
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&["../protos/cynic.proto"], &["../protos"])?;
    Ok(())
}
