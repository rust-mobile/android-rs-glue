mod compile;
use crate::config::{AndroidConfig, AndroidTargetConfig};
use cargo::core::{Target, TargetKind, Workspace};
use cargo::util::process_builder::process;
use cargo::util::CargoResult;
use clap::ArgMatches;
use failure::format_err;
use std::collections::{BTreeMap, HashSet};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::{env, fs};

pub use compile::AndroidAbi;

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
    let root_build_dir = get_root_build_directory(workspace, config);
    let (targets, abis) =
        compile::build_static_libraries(workspace, config, options, &root_build_dir)?;
    build_apks(config, &root_build_dir, &targets, &abis)
}

/// Returns the directory in which all cargo apk artifacts for the current
/// debug/release configuration should be produced.
fn get_root_build_directory(workspace: &Workspace, config: &AndroidConfig) -> PathBuf {
    let android_artifacts_dir = workspace
        .target_dir()
        .join("android-artifacts")
        .into_path_unlocked();

    if config.release {
        android_artifacts_dir.join("release")
    } else {
        android_artifacts_dir.join("debug")
    }
}

fn build_apks(
    config: &AndroidConfig,
    root_build_dir: &PathBuf,
    targets: &HashSet<Target>,
    abis: &[AndroidAbi],
) -> CargoResult<BuildResult> {
    let abis_str = abis.join(" ");

    // Create directory to hold final APKs which are signed using the debug key
    let final_apk_dir = root_build_dir.join("apk");
    fs::create_dir_all(&final_apk_dir)?;

    // Paths of created APKs
    let mut target_to_apk_map = BTreeMap::new();

    // Build an APK for each cargo target
    for target in targets.iter() {
        let target_directory = match target.kind() {
            TargetKind::Bin => root_build_dir.join("bin"),
            TargetKind::ExampleBin => root_build_dir.join("examples"),
            _ => unreachable!("Unexpected target kind"),
        };

        let target_directory = target_directory.join(target.name());
        fs::create_dir_all(&target_directory)?;

        // Run ndk-build
        build_makefiles(&target_directory, target, &abis_str, config)?;

        let mut ndk_build_cmd = if cfg!(target_os = "windows") {
            let mut pb = process("cmd");
            let ndk_build_path = config.ndk_path.join("build/ndk-build.cmd");
            pb.arg("/C").arg(ndk_build_path);
            pb
        } else {
            let ndk_build_path = config.ndk_path.join("build/ndk-build");
            process(ndk_build_path)
        };

        ndk_build_cmd
            .arg("NDK_LIBS_OUT=./lib")
            .cwd(&target_directory)
            .exec()?;

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

        // Add binaries
        for abi in abis {
            let so_path = format!("lib/{}/lib{}.so", abi, target.name());
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
        let mut apksigner_cmd = if cfg!(target_os = "windows") {
            let mut pb = process("cmd");
            let apksigner_path = build_tools_path.join("apksigner.bat");
            pb.arg("/C").arg(apksigner_path);
            pb
        } else {
            let apksigner_path = build_tools_path.join("apksigner");
            process(apksigner_path)
        };

        apksigner_cmd
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

fn build_makefiles(
    target_directory: &Path,
    target: &Target,
    abis: &str,
    config: &AndroidConfig,
) -> CargoResult<()> {
    let output_directory = target_directory.join("jni");
    fs::create_dir_all(&output_directory)?;

    // Write Android.mk
    let file = output_directory.join("Android.mk");
    let mut file = File::create(&file)?;

    writeln!(
        file,
        r#"LOCAL_PATH := $(call my-dir)

# Define module for static library built by rustc
include $(CLEAR_VARS)
LOCAL_MODULE := rustlib
LOCAL_SRC_FILES := ../../../$(TARGET_ARCH_ABI)/build/lib{target_library_name}.a
include $(PREBUILT_STATIC_LIBRARY)

# Build the application
include $(CLEAR_VARS)

LOCAL_MODULE    := {target_name}
LOCAL_SRC_FILES :=
LOCAL_LDLIBS    := -llog -landroid
LOCAL_STATIC_LIBRARIES := android_native_app_glue rustlib
NDK_LIBS_OUT := ./lib

include $(BUILD_SHARED_LIBRARY)

$(call import-module,android/native_app_glue)"#,
        target_library_name = target.name().replace("-", "_"),
        target_name = target.name()
    )?;

    // Write Application.mk
    let file = output_directory.join("Application.mk");
    let mut file = File::create(&file)?;

    let app_optim = if config.release { "release" } else { "debug" };

    write!(
        file,
        r#"APP_ABI := {}
APP_PLATFORM := android-{}
APP_OPTIM := {}"#,
        abis, config.min_sdk_version, app_optim
    )?;

    Ok(())
}
