use std::path::Path;

use ops::build;
use config::AndroidConfig;

pub fn install(manifest_path: &Path, config: &AndroidConfig) {
    /*let build_result = build::build(manifest_path, config);

    let adb = config.sdk_path.join("platform-tools/adb");

    TermCmd::new("Installing apk to the device", &adb)
        .arg("install")
        .arg("-r")      // TODO: let user choose
        .arg(&build_result.apk_path)
        .execute();*/
    unimplemented!()
}
