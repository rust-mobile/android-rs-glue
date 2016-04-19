extern crate rustc_serialize;

use std::env;
use std::path::Path;
use std::path::PathBuf;
use std::process::exit;
use std::process::Command;

mod build;
mod config;
mod install;

fn main() {
    let command = env::args().skip(2).next();

    let current_manifest = current_manifest_path();

    // Fetching the configuration for the build.
    let config = config::load();

    if command.as_ref().map(|s| &s[..]) == Some("install") {
        install::install(&current_manifest, &config);
    } else {
        build::build(&current_manifest, &config);
    }
}

fn current_manifest_path() -> PathBuf {
    let output = Command::new("cargo").arg("locate-project").output().unwrap();

    if !output.status.success() {
        if let Some(code) = output.status.code() {
            exit(code);
        } else {
            exit(-1);
        }
    }

    #[derive(RustcDecodable)]
    struct Data { root: String }
    let stdout = String::from_utf8(output.stdout).unwrap();
    let decoded: Data = rustc_serialize::json::decode(&stdout).unwrap();
    Path::new(&decoded.root).to_owned()
}
