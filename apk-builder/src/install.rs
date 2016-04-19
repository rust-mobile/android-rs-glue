use std::path::Path;
use std::process::exit;
use std::process::{Command, Stdio};

use build;
use config::Config;

pub fn install(manifest_path: &Path, config: &Config) {
    build::build(manifest_path, config);

    let adb = config.sdk_path.join("platform-tools/adb");
    let apk_path = Path::new("target/android-artifacts/build/bin/rust-android-debug.apk");      // TODO:

    if Command::new(&adb)
        .arg("install")
        .arg("-r")      // TODO: let user choose
        .arg(apk_path)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status().unwrap().code().unwrap() != 0
    {
        exit(1);
    }
}
