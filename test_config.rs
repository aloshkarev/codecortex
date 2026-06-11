use cortex_core::CortexConfig;
fn main() {
    let path = "/tmp/cortex_test_home/.cortex/config.toml";
    let raw = std::fs::read_to_string(path).unwrap();
    match toml::from_str::<CortexConfig>(&raw) {
        Ok(c) => println!("OK: {:?}", c.falkordb_uri),
        Err(e) => println!("ERROR: {}", e),
    }
}
