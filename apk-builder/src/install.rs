use std::path::Path;
use std::process::exit;
use std::process::{Command, Stdio};

use build;
use config::Config;

pub fn install(manifest_path: &Path, config: &Config) {
    let build_result = build::build(manifest_path, config);

    let adb = config.sdk_path.join("platform-tools/adb");

    if Command::new(&adb)
        .arg("install")
        .arg("-r")      // TODO: let user choose
        .arg(&build_result.apk_path)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status().unwrap().code().unwrap() != 0
    {
        exit(1);
    }
}
