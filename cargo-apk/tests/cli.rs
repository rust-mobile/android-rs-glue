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

#[test]
fn check_inner_attributes_test() {
    build_test("inner_attributes");
}

fn build_example(directory_name: &str) {
    build_package(&format!("../examples/{}/", directory_name));
}

fn build_test(directory_name: &str) {
    build_package(&format!("./tests/{}/", directory_name));
}

fn build_package(package_path: &str) {
    let mut cmd = Command::cargo_bin("cargo-apk").unwrap();
    cmd.arg("build");
    cmd.arg("--all-targets");
    cmd.current_dir(package_path);
    cmd.assert().success();
}
