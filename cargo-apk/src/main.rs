#![deny(warnings)]

extern crate cargo;
extern crate rustc_serialize;
extern crate term;
extern crate toml;

use std::env;

use cargo::core::Workspace;
use cargo::ops::MessageFormat;
use cargo::util::Config as CargoConfig;
use cargo::util::important_paths::find_root_manifest_for_wd;
use cargo::util::process_builder::process;

mod config;
mod ops;

fn main() {
    let cargo_config = CargoConfig::default().unwrap();

    let args = env::args().collect::<Vec<_>>();

    let err = match args.get(2).map(|a| &a[..]) {
        Some("build") => {
            cargo::call_main_without_stdin(execute_build, &cargo_config, BUILD_USAGE, &args, false)
        },
        Some("install") => {
            cargo::call_main_without_stdin(execute_install, &cargo_config, INSTALL_USAGE, &args, false)
        },
        Some("logcat") => {
            cargo::call_main_without_stdin(execute_logcat, &cargo_config, LOGCAT_USAGE, &args, false)
        },
        Some(opt) if opt.starts_with("-") => {
            println!("Note: `cargo apk [options]` is deprecated ; use `cargo apk build [options]` instead");
            let mut tweaked_args = args.clone();
            tweaked_args.insert(2, "build".to_owned());
            cargo::call_main_without_stdin(execute_build, &cargo_config, BUILD_USAGE, &tweaked_args, false)
        },
        None => {
            println!("Note: `cargo apk [options]` is deprecated ; use `cargo apk build [options]` instead");
            let mut tweaked_args = args.clone();
            tweaked_args.insert(2, "build".to_owned());
            cargo::call_main_without_stdin(execute_build, &cargo_config, BUILD_USAGE, &tweaked_args, false)
        },
        Some(_) => {
            // TODO: do more properly
            println!("Try the following commands: build, install, logcat");
            Ok(())
        }
    };

    match err {
        Ok(_) => (),
        Err(err) => cargo::exit_with_error(err, &mut *cargo_config.shell())
    }
}

pub fn execute_build(options: Options, cargo_config: &CargoConfig) -> cargo::CliResult {
    cargo_config.configure(options.flag_verbose,
                           options.flag_quiet,
                           &options.flag_color,
                           options.flag_frozen,
                           options.flag_locked)?;


    let root_manifest = find_root_manifest_for_wd(options.flag_manifest_path.clone(),
                                                  cargo_config.cwd())?;

    let workspace = Workspace::new(&root_manifest, &cargo_config)?;

    let mut android_config = config::load(&workspace, &options.flag_package)?;
    android_config.release = options.flag_release;

    ops::build(&workspace, &android_config, &options)?;
    Ok(())
}

pub fn execute_install(options: Options, cargo_config: &CargoConfig) -> cargo::CliResult {
    cargo_config.configure(options.flag_verbose,
                           options.flag_quiet,
                           &options.flag_color,
                           options.flag_frozen,
                           options.flag_locked)?;

    let root_manifest = find_root_manifest_for_wd(options.flag_manifest_path.clone(),
                                                  cargo_config.cwd())?;

    let workspace = Workspace::new(&root_manifest, &cargo_config)?;

    let mut android_config = config::load(&workspace, &options.flag_package)?;
    android_config.release = options.flag_release;

    ops::install(&workspace, &android_config, &options)?;
    Ok(())
}

pub fn execute_logcat(options: LogcatOptions, cargo_config: &CargoConfig) -> cargo::CliResult {
    cargo_config.configure(options.flag_verbose,
                           options.flag_quiet,
                           &options.flag_color,
                           options.flag_frozen,
                           options.flag_locked)?;

    let root_manifest = find_root_manifest_for_wd(options.flag_manifest_path.clone(),
                                                  cargo_config.cwd())?;

    let workspace = Workspace::new(&root_manifest, &cargo_config)?;

    let android_config = config::load(&workspace, &options.flag_package)?;
    
    workspace.config().shell().say("Starting logcat", 10)?;
    let adb = android_config.sdk_path.join("platform-tools/adb");
    process(&adb)
        .arg("logcat")
        .exec()?;

    Ok(())
}

#[derive(RustcDecodable)]
pub struct Options {
    flag_bin: Option<String>,
    flag_example: Option<String>,
    flag_package: Option<String>,
    flag_jobs: Option<u32>,
    flag_features: Vec<String>,
    flag_all_features: bool,
    flag_no_default_features: bool,
    flag_manifest_path: Option<String>,
    flag_verbose: u32,
    flag_quiet: Option<bool>,
    flag_color: Option<String>,
    flag_message_format: MessageFormat,
    flag_release: bool,
    flag_frozen: bool,
    flag_locked: bool,
}

#[derive(RustcDecodable)]
pub struct LogcatOptions {
    flag_package: Option<String>,
    flag_manifest_path: Option<String>,
    flag_verbose: u32,
    flag_quiet: Option<bool>,
    flag_color: Option<String>,
    flag_frozen: bool,
    flag_locked: bool,
}

const BUILD_USAGE: &'static str = r#"
Usage:
    cargo apk build [options]

Options:
    -h, --help                   Print this message
    --bin NAME                   Name of the bin target to run
    --example NAME               Name of the example target to run
    -p SPEC, --package SPEC      Package with the target to run
    -j N, --jobs N               Number of parallel jobs, defaults to # of CPUs
    --release                    Build artifacts in release mode, with optimizations
    --features FEATURES          Space-separated list of features to also build
    --all-features               Build all available features
    --no-default-features        Do not build the `default` feature
    --manifest-path PATH         Path to the manifest to execute
    -v, --verbose ...            Use verbose output (-vv very verbose/build.rs output)
    -q, --quiet                  No output printed to stdout
    --color WHEN                 Coloring: auto, always, never
    --message-format FMT         Error format: human, json [default: human]
    --frozen                     Require Cargo.lock and cache are up to date
    --locked                     Require Cargo.lock is up to date

Does the same as `cargo build`.
"#;

const INSTALL_USAGE: &'static str = r#"
Usage:
    cargo apk install [options]

Options:
    -h, --help                   Print this message
    --bin NAME                   Name of the bin target to run
    --example NAME               Name of the example target to run
    -p SPEC, --package SPEC      Package with the target to run
    -j N, --jobs N               Number of parallel jobs, defaults to # of CPUs
    --release                    Build artifacts in release mode, with optimizations
    --features FEATURES          Space-separated list of features to also build
    --all-features               Build all available features
    --no-default-features        Do not build the `default` feature
    --manifest-path PATH         Path to the manifest to execute
    -v, --verbose ...            Use verbose output (-vv very verbose/build.rs output)
    -q, --quiet                  No output printed to stdout
    --color WHEN                 Coloring: auto, always, never
    --message-format FMT         Error format: human, json [default: human]
    --frozen                     Require Cargo.lock and cache are up to date
    --locked                     Require Cargo.lock is up to date

Does the same as `cargo build`.
"#;

const LOGCAT_USAGE: &'static str = r#"
Usage:
    cargo apk logcat [options]

Options:
    -h, --help                   Print this message
    -p SPEC, --package SPEC      Package with the target to run
    --manifest-path PATH         Path to the manifest to execute
    -v, --verbose ...            Use verbose output (-vv very verbose/build.rs output)
    -q, --quiet                  No output printed to stdout
    --color WHEN                 Coloring: auto, always, never
    --frozen                     Require Cargo.lock and cache are up to date
    --locked                     Require Cargo.lock is up to date

Starts `adb logcat`.
"#;
