mod compile;
mod targets;
pub mod tempfile;
mod util;

use crate::config::{AndroidConfig, AndroidTargetConfig};
use cargo::core::{Target, TargetKind, Workspace};
use cargo::util::process_builder::process;
use cargo::util::CargoResult;
use clap::ArgMatches;
use failure::format_err;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::{env, fs};

use crate::ops::build::compile::SharedLibraries;

#[derive(Debug)]
pub struct BuildResult {
    /// Mapping from target kind and target name to the built APK
    pub target_to_apk_map: BTreeMap<(TargetKind, String), PathBuf>,
}

pub fn build(
    workspace: &Workspace,
    config: &AndroidConfig,
    options: &ArgMatches,
) -> CargoResult<BuildResult> {
    let root_build_dir = util::get_root_build_directory(workspace, config);
    let shared_libraries =
        compile::build_shared_libraries(workspace, config, options, &root_build_dir)?;
    build_apks(config, &root_build_dir, shared_libraries)
}

fn build_apks(
    config: &AndroidConfig,
    root_build_dir: &PathBuf,
    shared_libraries: SharedLibraries,
) -> CargoResult<BuildResult> {
    // Create directory to hold final APKs which are signed using the debug key
    let final_apk_dir = root_build_dir.join("apk");
    fs::create_dir_all(&final_apk_dir)?;

    // Paths of created APKs
    let mut target_to_apk_map = BTreeMap::new();

    // Build an APK for each cargo target
    for (target, shared_libraries) in shared_libraries.shared_libraries.iter_all() {
        let target_directory = util::get_target_directory(root_build_dir, target)?;
        fs::create_dir_all(&target_directory)?;

        // Determine Target Configuration
        let target_config = config.resolve((target.kind().to_owned(), target.name().to_owned()))?;

        //
        // Run commands to produce APK
        //
        build_manifest(&target_directory, &config, &target_config, &target)?;

        let build_tools_path = config
            .sdk_path
            .join("build-tools")
            .join(&config.build_tools_version);
        let aapt_path = build_tools_path.join("aapt");
        let aapt2_path = build_tools_path.join("aapt2");
        let zipalign_path = build_tools_path.join("zipalign");

        // Compile resources
        let compiled_resources_filename = "resources.zip";
        if let Some(res_path) = &target_config.res_path {
            process(&aapt2_path)
                .arg("compile")
                .arg("--dir")
                .arg(res_path)
                .arg("-o")
                .arg(compiled_resources_filename)
                .cwd(&target_directory)
                .exec()?;
        }

        // Create unaligned APK which includes resources
        let unaligned_apk_name = format!("{}_unaligned.apk", target.name());
        let mut aapt2_link_cmd = process(&aapt2_path);
        aapt2_link_cmd
            .arg("link")
            .arg("-o")
            .arg(&unaligned_apk_name)
            .arg("--manifest")
            .arg("AndroidManifest.xml")
            .arg("-I")
            .arg(&config.android_jar_path);

        if target_config.res_path.is_some() {
            aapt2_link_cmd.arg(compiled_resources_filename);
        }

        // Link assets
        if let Some(assets_path) = &target_config.assets_path {
            aapt2_link_cmd.arg("-A").arg(assets_path);
        }

        aapt2_link_cmd.cwd(&target_directory).exec()?;

        // Add shared libraries to the APK
        for shared_library in shared_libraries {
            // Copy the shared library to the appropriate location in the target directory and with the appropriate name
            // Note: that the type of slash used matters. This path is passed to aapt and the shared library
            // will not load if backslashes are used.
            let so_path = format!(
                "lib/{}/{}",
                &shared_library.abi.android_abi(),
                shared_library.filename
            );

            let target_shared_object_path = target_directory.join(&so_path);
            fs::create_dir_all(target_shared_object_path.parent().unwrap())?;
            fs::copy(&shared_library.path, target_shared_object_path)?;

            // Add to the APK
            process(&aapt_path)
                .arg("add")
                .arg(&unaligned_apk_name)
                .arg(so_path)
                .cwd(&target_directory)
                .exec()?;
        }

        // Determine the directory in which to place the aligned and signed APK
        let target_apk_directory = match target.kind() {
            TargetKind::Bin => final_apk_dir.clone(),
            TargetKind::ExampleBin => final_apk_dir.join("examples"),
            _ => unreachable!("Unexpected target kind"),
        };
        fs::create_dir_all(&target_apk_directory)?;

        // Align apk
        let final_apk_path = target_apk_directory.join(format!("{}.apk", target.name()));
        process(&zipalign_path)
            .arg("-f")
            .arg("-v")
            .arg("4")
            .arg(&unaligned_apk_name)
            .arg(&final_apk_path)
            .cwd(&target_directory)
            .exec()?;

        // Find or generate a debug keystore for signing the APK
        // We use the same debug keystore as used by the Android SDK. If it does not exist,
        // then we create it using keytool which is part of the JRE/JDK
        let keystore_path = dirs::home_dir()
            .ok_or_else(|| format_err!("Unable to determine home directory"))?
            .join(".android/debug.keystore");
        if !keystore_path.exists() {
            // Generate key
            let keytool_filename = if cfg!(target_os = "windows") {
                "keytool.exe"
            } else {
                "keytool"
            };

            let keytool_path = find_java_executable(keytool_filename)?;
            process(keytool_path)
                .arg("-genkey")
                .arg("-v")
                .arg("-keystore")
                .arg(&keystore_path)
                .arg("-storepass")
                .arg("android")
                .arg("-alias")
                .arg("androidebugkey")
                .arg("-keypass")
                .arg("android")
                .arg("-dname")
                .arg("CN=Android Debug,O=Android,C=US")
                .arg("-keyalg")
                .arg("RSA")
                .arg("-keysize")
                .arg("2048")
                .arg("-validity")
                .arg("10000")
                .cwd(root_build_dir)
                .exec()?;
        }

        // Sign the APK with the development certificate
        util::script_process(
            build_tools_path.join(format!("apksigner{}", util::EXECUTABLE_SUFFIX_BAT)),
        )
        .arg("sign")
        .arg("--ks")
        .arg(keystore_path)
        .arg("--ks-pass")
        .arg("pass:android")
        .arg(&final_apk_path)
        .cwd(&target_directory)
        .exec()?;

        target_to_apk_map.insert(
            (target.kind().to_owned(), target.name().to_owned()),
            final_apk_path,
        );
    }

    Ok(BuildResult { target_to_apk_map })
}

/// Find an executable that is part of the Java SDK
fn find_java_executable(name: &str) -> CargoResult<PathBuf> {
    // Look in PATH
    env::var_os("PATH")
        .and_then(|paths| {
            env::split_paths(&paths)
                .filter_map(|path| {
                    let filepath = path.join(name);
                    if fs::metadata(&filepath).is_ok() {
                        Some(filepath)
                    } else {
                        None
                    }
                })
                .next()
        })
        .or_else(||
            // Look in JAVA_HOME
            env::var_os("JAVA_HOME").and_then(|java_home| {
                let filepath = PathBuf::from(java_home).join("bin").join(name);
                if filepath.exists() {
                    Some(filepath)
                } else {
                    None
                }
            }))
        .ok_or_else(|| {
            format_err!(
                "Unable to find executable: '{}'. Configure PATH or JAVA_HOME with the path to the JRE or JDK.",
                name
            )
        })
}

fn build_manifest(
    path: &Path,
    config: &AndroidConfig,
    target_config: &AndroidTargetConfig,
    target: &Target,
) -> CargoResult<()> {
    let file = path.join("AndroidManifest.xml");
    let mut file = File::create(&file)?;

    // Building application attributes
    let application_attrs = format!(
        r#"
            android:hasCode="false" android:label="{0}"{1}{2}{3}"#,
        target_config.package_label,
        target_config
            .package_icon
            .as_ref()
            .map_or(String::new(), |a| format!(
                r#"
            android:icon="{}""#,
                a
            )),
        if target_config.fullscreen {
            r#"
            android:theme="@android:style/Theme.DeviceDefault.NoActionBar.Fullscreen""#
        } else {
            ""
        },
        target_config
            .application_attributes
            .as_ref()
            .map_or(String::new(), |a| a.replace("\n", "\n            "))
    );

    // Build activity attributes
    let activity_attrs = format!(
        r#"
                android:name="android.app.NativeActivity"
                android:label="{0}"
                android:configChanges="orientation|keyboardHidden|screenSize" {1}"#,
        target_config.package_label,
        target_config
            .activity_attributes
            .as_ref()
            .map_or(String::new(), |a| a.replace("\n", "\n                "))
    );

    let uses_features = target_config
        .features
        .iter()
        .map(|f| {
            format!(
                "\n\t<uses-feature android:name=\"{}\" android:required=\"{}\" {}/>",
                f.name,
                f.required,
                f.version
                    .as_ref()
                    .map_or(String::new(), |v| format!(r#"android:version="{}""#, v))
            )
        })
        .collect::<Vec<String>>()
        .join(", ");

    let uses_permissions = target_config
        .permissions
        .iter()
        .map(|f| {
            format!(
                "\n\t<uses-permission android:name=\"{}\" {max_sdk_version}/>",
                f.name,
                max_sdk_version = f.max_sdk_version.map_or(String::new(), |v| format!(
                    r#"android:maxSdkVersion="{}""#,
                    v
                ))
            )
        })
        .collect::<Vec<String>>()
        .join(", ");

    // Write final AndroidManifest
    writeln!(
        file, r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
        package="{package}"
        android:versionCode="{version_code}"
        android:versionName="{version_name}">
    <uses-sdk android:targetSdkVersion="{targetSdkVersion}" android:minSdkVersion="{minSdkVersion}" />
    <uses-feature android:glEsVersion="{glEsVersion}" android:required="true"></uses-feature>{uses_features}{uses_permissions}
    <application {application_attrs} >
        <activity {activity_attrs} >
            <meta-data android:name="android.app.lib_name" android:value="{target_name}" />
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
</manifest>"#,
        package = target_config.package_name.replace("-", "_"),
        version_code = target_config.version_code,
        version_name = target_config.version_name,
        targetSdkVersion = config.target_sdk_version,
        minSdkVersion = config.min_sdk_version,
        glEsVersion = format!("0x{:04}{:04}", target_config.opengles_version_major, target_config.opengles_version_minor),
        uses_features = uses_features,
        uses_permissions = uses_permissions,
        application_attrs = application_attrs,
        activity_attrs = activity_attrs,
        target_name = target.name(),
    )?;

    Ok(())
}
