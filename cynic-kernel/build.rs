fn main() -> Result<(), Box<dyn std::error::Error>> {
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
