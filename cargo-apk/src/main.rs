extern crate cargo;
extern crate rustc_serialize;
extern crate term;
extern crate toml;

use std::env;

use cargo::core::Workspace;
use cargo::util::Config as CargoConfig;
use cargo::util::important_paths::find_root_manifest_for_wd;

mod build;
mod config;
mod install;
mod termcmd;

fn main() {
    let cargo_config = CargoConfig::default().unwrap();
    // TODO: call cargo_config.config(...)
    let root_manifest = find_root_manifest_for_wd(None /* TODO */, cargo_config.cwd()).unwrap();

    let workspace = Workspace::new(&root_manifest, &cargo_config).unwrap();
    let current_package = workspace.current().unwrap();

    let command = env::args().skip(2).next();

    // Fetching the configuration for the build.
    let mut config = config::load(current_package.manifest_path());
    config.release = env::args().any(|s| &s[..] == "--release");
    if let Some(target_arg_index) = env::args().position(|s| &s[..] == "--bin") {
        config.target = env::args().skip(target_arg_index + 1).next();
    }

    if command.as_ref().map(|s| &s[..]) == Some("install") {
        install::install(current_package.manifest_path(), &config);
    } else {
        build::build(current_package.manifest_path(), &config);
    }
}
