//! Build script: generates tonic/prost code from `proto/synapse.proto`.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_root = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?)
        .join("..") // crates/tui → crates/
        .join("..") // crates/ → workspace root
        .join("proto");

    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_protos(&[proto_root.join("synapse.proto")], &[&proto_root])?;

    println!("cargo:rerun-if-changed=../../proto/synapse.proto");
    Ok(())
}
