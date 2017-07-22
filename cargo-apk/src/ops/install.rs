use cargo::core::Workspace;
use cargo::util::errors::CargoError;
use cargo::util::process_builder::process;
use ops::build;
use config::AndroidConfig;
use Options;

pub fn install(workspace: &Workspace, config: &AndroidConfig, options: &Options)
               -> Result<(), CargoError>
{
    let build_result = build::build(workspace, config, options)?;

    let adb = config.sdk_path.join("platform-tools/adb");

    workspace.config().shell().say("Installing apk to the device", 10)?;
    process(&adb)
        .arg("install")
        .arg("-r")      // TODO: let user choose
        .arg(&build_result.apk_path)
        .exec()?;

    Ok(())
}
