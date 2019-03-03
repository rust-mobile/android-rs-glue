use cargo::core::Workspace;
use cargo::util::errors::CargoError;
use cargo::util::process_builder::process;
use clap::ArgMatches;
use config::AndroidConfig;
use ops::build;

pub fn install(
    workspace: &Workspace,
    config: &AndroidConfig,
    options: &ArgMatches,
) -> Result<(), CargoError> {
    let build_result = build::build(workspace, config, options)?;

    let adb = config.sdk_path.join("platform-tools/adb");

    drop(writeln!(
        workspace.config().shell().err(),
        "Installing apk to the device"
    ));
    process(&adb)
        .arg("install")
        .arg("-r") // TODO: let user choose
        .arg(&build_result.apk_path)
        .exec()?;

    Ok(())
}
