//! Validator `validate_build` resolves TWAG (CMake) and rdiameter (Cargo) layouts.

use cortex_core::A2aValidateConfig;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn twag_layout_resolves_to_build_sh() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("CMakeLists.txt"), "cmake_minimum_required(VERSION 3.16)\n")
        .unwrap();
    fs::write(dir.path().join("build.sh"), "#!/bin/sh\nexit 0\n").unwrap();

    let plan = A2aValidateConfig::default()
        .resolve(dir.path())
        .expect("plan");
    assert_eq!(plan.program, "./build.sh");
    assert_eq!(plan.cwd, dir.path());
}

#[test]
fn rdiameter_layout_resolves_to_cargo_check() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"rdiameter\"\n").unwrap();

    let plan = A2aValidateConfig::default()
        .resolve(dir.path())
        .expect("plan");
    assert_eq!(plan.program, "cargo");
    assert_eq!(plan.args, vec!["check", "--quiet"]);
}

#[test]
fn rdiameter_from_twag_monorepo_via_working_directory() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("CMakeLists.txt"), "cmake_minimum_required(VERSION 3.16)\n")
        .unwrap();
    fs::write(dir.path().join("build.sh"), "#!/bin/sh\nexit 0\n").unwrap();
    let rdiameter = dir.path().join("third_party/tngf_cp/rdiameter");
    fs::create_dir_all(&rdiameter).unwrap();
    fs::write(
        rdiameter.join("Cargo.toml"),
        "[package]\nname = \"rdiameter\"\n",
    )
    .unwrap();

    let cfg = A2aValidateConfig {
        command: Vec::new(),
        working_directory: Some(PathBuf::from("third_party/tngf_cp/rdiameter")),
    };
    let plan = cfg.resolve(dir.path()).expect("plan");
    assert_eq!(plan.program, "cargo");
    assert_eq!(plan.cwd, rdiameter);
}
