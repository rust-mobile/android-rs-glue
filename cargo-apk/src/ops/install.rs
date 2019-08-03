use super::BuildResult;
use crate::config::AndroidConfig;
use crate::ops::build;
use cargo::core::Workspace;
use cargo::util::process_builder::process;
use cargo::util::CargoResult;
use clap::ArgMatches;

pub fn install(
    workspace: &Workspace,
    config: &AndroidConfig,
    options: &ArgMatches,
) -> CargoResult<BuildResult> {
    let build_result = build::build(workspace, config, options)?;

    let adb = config.sdk_path.join("platform-tools/adb");

    for apk_path in build_result.target_to_apk_map.values() {
        drop(writeln!(
            workspace.config().shell().err(),
            "Installing apk '{}' to the device",
            apk_path.file_name().unwrap().to_string_lossy()
        ));

        process(&adb)
            .arg("install")
            .arg("-r")
            .arg(apk_path)
            .exec()?;
    }

    Ok(build_result)
}
