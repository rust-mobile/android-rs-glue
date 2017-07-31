use cargo::core::Workspace;
use cargo::util::errors::CargoError;
use cargo::util::process_builder::process;
use ops::install;
use config::AndroidConfig;
use Options;

pub fn run(workspace: &Workspace, config: &AndroidConfig, options: &Options)
               -> Result<(), CargoError>
{
    let _build_result = install::install(workspace, config, options)?;

    let adb = config.sdk_path.join("platform-tools/adb");

    // Found it by doing this :
    //     adb shell "cmd package resolve-activity --brief com.author.myproject | tail -n 1"
    let activity_path = format!(
        "{}/rust.{}.MainActivity", 
        config.package_name.replace("-", "_"),
        config.project_name.replace("-", "_")
    );

    workspace.config().shell().say("Running apk", 10)?;
    process(&adb)
        .arg("shell").arg("am").arg("start")
        .arg("-a").arg("android.intent.action.MAIN")
        .arg("-n").arg(&activity_path)
        .exec()?;

    Ok(())
}
