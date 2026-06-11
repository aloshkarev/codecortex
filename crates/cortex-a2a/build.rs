use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);
    let docs = manifest_dir.join("../../docs");
    let proto = docs.join("a2a.proto");

    println!("cargo:rerun-if-changed={}", proto.display());
    println!("cargo:rerun-if-changed={}", docs.join("google").display());

    let descriptor_path = out_dir.join("a2a_descriptor.bin");

    let mut config = prost_build::Config::new();
    config.file_descriptor_set_path(&descriptor_path);
    config.extern_path(".google.protobuf.Struct", "::prost_types::Struct");
    config.extern_path(".google.protobuf.Timestamp", "::prost_types::Timestamp");
    config.extern_path(".google.protobuf.Value", "::prost_types::Value");
    config.compile_protos(&[proto], &[docs])?;

    // pbjson serde types generated for contract drift detection (not included in proto.rs until WKT serde unifies).
    let descriptor_set = std::fs::read(descriptor_path)?;
    let _ = pbjson_build::Builder::new()
        .register_descriptors(&descriptor_set)?
        .extern_path(".google.protobuf.Struct", "::pbjson_types::Struct")
        .extern_path(".google.protobuf.Timestamp", "::pbjson_types::Timestamp")
        .extern_path(".google.protobuf.Value", "::pbjson_types::Value")
        .build(&[".lf.a2a.v1"]);

    Ok(())
}
