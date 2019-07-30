use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn check_advanced_example() {
    build_example("advanced");
}

#[test]
fn check_basic_example() {
    build_example("basic");
}

#[test]
fn check_multiple_targets_example() {
    build_example("multiple_targets");
}

#[test]
fn check_use_asset_example() {
    build_example("use_assets");
}

#[test]
fn check_use_icon_example() {
    build_example("use_icon");
}

fn build_example(directory_name: &str) {
    let example_path = format!("../examples/{}/", directory_name);
    let mut cmd = Command::cargo_bin("cargo-apk").unwrap();
    cmd.arg("build");
    cmd.arg("--all-targets");
    cmd.current_dir(example_path);
    cmd.assert().success();
}
