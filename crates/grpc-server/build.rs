//! Build script: generates tonic/prost code from `proto/synapse.proto` via `tonic-prost-build`.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_root = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?)
        .join("..") // crates/grpc-server → crates/
        .join("..") // crates/ → workspace root
        .join("proto");

    tonic_prost_build::configure()
        .build_server(true)
        .build_client(false)
        .compile_protos(
            &[proto_root.join("synapse.proto")],
            std::slice::from_ref(&proto_root),
        )?;

    println!("cargo:rerun-if-changed=../../proto/synapse.proto");
    Ok(())
}
