fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let docs = manifest_dir.join("../../docs");
    let proto = docs.join("a2a.proto");

    println!("cargo:rerun-if-changed={}", proto.display());
    println!("cargo:rerun-if-changed={}", docs.join("google").display());

    tonic_prost_build::configure()
        .build_server(true)
        .build_client(false)
        .extern_path(".google.protobuf.Struct", "::prost_types::Struct")
        .extern_path(".google.protobuf.Timestamp", "::prost_types::Timestamp")
        .extern_path(".google.protobuf.Value", "::prost_types::Value")
        .compile_protos(&[proto], &[docs])?;
    Ok(())
}
