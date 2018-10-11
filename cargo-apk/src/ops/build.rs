use std::os;
use std::collections::{HashSet, HashMap};
use std::env;
use std::fs;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::iter::FromIterator;
use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use cargo::ops;
use cargo::core::Workspace;
use cargo::util::errors::CargoError;
use cargo::util::process_builder::process;

use config::AndroidConfig;
use Options;

pub struct BuildResult {
    /// The absolute path where the apk is located.
    pub apk_path: PathBuf,
}

pub fn build(workspace: &Workspace, config: &AndroidConfig, options: &Options)
            -> Result<BuildResult, CargoError>
{
    // First we detect whether `gradle` works.
    match Command::new(&config.gradle_command).arg("-v").stdout(Stdio::null()).status() {
        Ok(s) if s.success() => (),
        _ => {
            return Err(CargoError::from(r#"Could not execute `gradle`. Did you
                install it? (If already installed on windows with `gradle.bat`
                in your path, you must customise the gradle command to
                `gradle.bat` with the CARGO_APK_GRADLE_COMMAND environment
                variable)."#));
        }
    }

    // Building the `android-artifacts` directory that will contain all the artifacts.
    // FIXME: don't use into_path_unlocked() but pass a Cargo::Filesystem everywhere
    let android_artifacts_dir = workspace.target_dir().join("android-artifacts").into_path_unlocked();
    build_android_artifacts_dir(workspace, &android_artifacts_dir, &config)?;

    let mut abi_libs: HashMap<&str, Vec<String>> = HashMap::new();

    for build_target in config.build_targets.iter() {
        assert_ne!(build_target, "app");
        let build_target_dir = android_artifacts_dir.join(build_target);

        // Finding the tools in the NDK.
        let (gcc_path, gxx_path, ar_path) = {
            let host_os = if cfg!(target_os = "windows") { "windows" }
                       else if cfg!(target_os = "linux") { "linux" }
                       else if cfg!(target_os = "macos") { "darwin" }
                       else { panic!("Unknown or incompatible host OS") };

            let target_arch = if build_target.starts_with("arm") { "arm-linux-androideabi" }
                       else if build_target.starts_with("aarch64") { "aarch64-linux-android" }
                       else if build_target.starts_with("i") { "x86" }
                       else if build_target.starts_with("x86_64") { "x86_64" }
                       else if build_target.starts_with("mipsel") { "mipsel-linux-android" }
                       // TODO: mips64
                       else { panic!("Unknown or incompatible build target: {}", build_target) };

            // Looks like the tools don't always share the prefix of the target arch
            // Just a macos issue?
            let tool_prefix = if build_target.starts_with("arm") { "arm-linux-androideabi" }
                       else if build_target.starts_with("aarch64") { "aarch64-linux-android" }
                       else if build_target.starts_with("i") { "i686-linux-android" }
                       else if build_target.starts_with("x86_64") { "x86_64-linux-android" }
                       else if build_target.starts_with("mipsel") { "mipsel-linux-android" }
                       // TODO: mips64
                       else { panic!("Unknown or incompatible build target: {}", build_target) };

            let base = config.ndk_path.join(format!("toolchains/{}-4.9/prebuilt/{}-x86_64", target_arch, host_os));
            (base.join(format!("bin/{}-gcc", tool_prefix)),
             base.join(format!("bin/{}-g++", tool_prefix)),
             base.join(format!("bin/{}-ar", tool_prefix)))
        };

        let gcc_sysroot_linker = {
            let arch = if build_target.starts_with("arm") { "arm" }
                       else if build_target.starts_with("aarch64") { "arm64" }
                       else if build_target.starts_with("i") { "x86" }
                       else if build_target.starts_with("x86_64") { "x86_64" }
                       else if build_target.starts_with("mips") { "mips" }
                       // TODO: mips64
                       else { panic!("Unknown or incompatible build target: {}", build_target) };
            config.ndk_path.join(format!("platforms/android-{v}/arch-{a}",
                                         v = config.android_version, a = arch))
        };

        // TODO Test and make compatible with older android NDKs if needed

        let gcc_sysroot = {
            config.ndk_path.join("sysroot")
        };

        let gcc_isystem = {
            let target_arch = if build_target.starts_with("arm") { "arm-linux-androideabi" }
                       else if build_target.starts_with("aarch64") { "aarch64-linux-android" }
                       else if build_target.starts_with("i") { "i686-linux-android" }
                       else if build_target.starts_with("x86_64") { "x86_64-linux-android" }
                       else if build_target.starts_with("mipsel") { "mipsel-linux-android" }
                       // TODO: mips64
                       else { panic!("Unknown or incompatible build target: {}", build_target) };
            config.ndk_path.join(format!("sysroot/usr/include/{}", target_arch))
        };

        // Create android cpu abi name
        let abi = if build_target.starts_with("armv7") { "armeabi-v7a" }
                  else if build_target.starts_with("arm") { "armeabi" }
                  else if build_target.starts_with("aarch64") { "arm64-v8a" }
                  else if build_target.starts_with("i") { "x86" }
                  else if build_target.starts_with("x86_64") { "x86_64" }
                  else if build_target.starts_with("mips") { "mips" }
                  // TODO: mips64
                  else { panic!("Unknown or incompatible build target: {}", build_target) };

        // Compiling android_native_app_glue.c
        {
            workspace.config().shell().say("Compiling android_native_app_glue.c", 10)?;
            let mut cmd = process(&gcc_path);
            cmd.arg(config.ndk_path.join("sources/android/native_app_glue/android_native_app_glue.c"))
               .arg("-c");
            if config.release {
                cmd.arg("-O3");
            }
            cmd.arg("-o").arg(build_target_dir.join("android_native_app_glue.o"))
               .arg("--sysroot").arg(&gcc_sysroot)
               .arg("-isystem").arg(&gcc_isystem)
               .exec()?;
        }

        // Compiling injected-glue
        let injected_glue_lib = {
            workspace.config().shell().say("Compiling injected-glue", 10)?;
            let mut cmd = workspace.config().rustc()?.process();
            cmd.arg(android_artifacts_dir.join("injected-glue/lib.rs"))
               .arg("--crate-type").arg("rlib");
            if config.release {
                cmd.arg("-C")
                   .arg("opt-level=3");
            }
            cmd.arg("--crate-name").arg("cargo_apk_injected_glue")
               .arg("--target").arg(build_target)
               .arg("--out-dir").arg(&build_target_dir);

            cmd.exec()?;

            let stdout = cmd.arg("--print").arg("file-names")
                            .exec_with_output()?;
            let stdout = String::from_utf8(stdout.stdout).unwrap();

            build_target_dir.join(stdout.lines().next().unwrap())
        };

        // Compiling glue_obj.rs
        {
            let mut file = File::create(build_target_dir.join("glue_obj.rs")).unwrap();
            file.write_all(&include_bytes!("../../glue_obj.rs")[..]).unwrap();
        }
        
        {
            workspace.config().shell().say("Compiling glue_obj", 10)?;
            let mut cmd = workspace.config().rustc()?.process();
            cmd.arg(build_target_dir.join("glue_obj.rs"))
               .arg("--crate-type").arg("staticlib");
            if config.release {
                cmd.arg("-C")
                   .arg("opt-level=3");
            }
            cmd.arg("--target").arg(build_target)
               .arg("--extern").arg(format!("cargo_apk_injected_glue={}", injected_glue_lib.to_string_lossy()))
               .arg("--emit").arg("obj")
               .arg("-o").arg(build_target_dir.join("glue_obj.o"))
               .exec()?;
        }

        // Directory where we will put the native libraries for gradle to pick them up.
        let native_libraries_dir = android_artifacts_dir.join(format!("app/lib/{}", abi));

        if fs::metadata(&native_libraries_dir).is_err() {
            fs::DirBuilder::new().recursive(true).create(&native_libraries_dir).unwrap();
        }

        // Compiling the crate thanks to `cargo rustc`. We set the linker to `linker_exe`, a hacky
        // linker that will tweak the options passed to `gcc`.
        {
            workspace.config().shell().say("Compiling crate", 10)?;

            // Set the current environment variables so that they are picked up by gcc-rs when
            // compiling.
            env::set_var(&format!("CC_{}", build_target), gcc_path.as_os_str());
            env::set_var(&format!("CXX_{}", build_target), gxx_path.as_os_str());
            env::set_var(&format!("AR_{}", build_target), ar_path.as_os_str());
            env::set_var(&format!("CFLAGS_{}", build_target), &format!("--sysroot {} -isysroot {} -isystem {}", gcc_sysroot_linker.to_string_lossy(), gcc_sysroot.to_string_lossy(), gcc_isystem.to_string_lossy()));
            env::set_var(&format!("CXXFLAGS_{}", build_target), &format!("--sysroot {} -isysroot {} -isystem {}", gcc_sysroot_linker.to_string_lossy(), gcc_sysroot.to_string_lossy(), gcc_isystem.to_string_lossy()));

            let extra_args = vec![
                "-C".to_owned(), format!("linker={}", android_artifacts_dir.join(if cfg!(target_os = "windows") { "linker_exe.exe" } else { "linker_exe" }).to_string_lossy()),
                "--extern".to_owned(), format!("cargo_apk_injected_glue={}", injected_glue_lib.to_string_lossy()),
                "-C".to_owned(),"link-arg=--cargo-apk-gcc".to_owned(),
                "-C".to_owned(), format!("link-arg={}", gcc_path.as_os_str().to_str().unwrap().to_owned()),
                "-C".to_owned(),"link-arg=--cargo-apk-gcc-sysroot".to_owned(),
                "-C".to_owned(), format!("link-arg={}", gcc_sysroot_linker.as_os_str().to_str().unwrap().to_owned()),
                "-C".to_owned(), "link-arg=--cargo-apk-native-app-glue".to_owned(),
                "-C".to_owned(), format!("link-arg={}", build_target_dir.join("android_native_app_glue.o").into_os_string().into_string().unwrap()),
                "-C".to_owned(), "link-arg=--cargo-apk-glue-obj".to_owned(),
                "-C".to_owned(), format!("link-arg={}", build_target_dir.join("glue_obj.o").into_os_string().into_string().unwrap()),
                "-C".to_owned(), "link-arg=--cargo-apk-glue-lib".to_owned(),
                "-C".to_owned(), format!("link-arg={}", injected_glue_lib.into_os_string().into_string().unwrap()),
                "-C".to_owned(), "link-arg=--cargo-apk-linker-output".to_owned(),
                "-C".to_owned(), format!("link-arg={}", native_libraries_dir.join("libmain.so").into_os_string().into_string().unwrap()),
                "-C".to_owned(), "link-arg=--cargo-apk-libs-path-output".to_owned(),
                "-C".to_owned(), format!("link-arg={}", build_target_dir.join("lib_paths").into_os_string().into_string().unwrap()),
                "-C".to_owned(), "link-arg=--cargo-apk-libs-output".to_owned(),
                "-C".to_owned(), format!("link-arg={}", build_target_dir.join("libs").into_os_string().into_string().unwrap()),
                // TODO Test and make compatible with different targets (tested only on armv7-linux-androideabi)
                "-C".to_owned(), "relocation-model=pic".to_owned(),
                "-C".to_owned(), "link-args=-no-pie".to_owned(),
                "-C".to_owned(), "link-args=-Wl,-Bsymbolic".to_owned(),
            ];

            let packages = Vec::from_iter(options.flag_package.iter().cloned());
            let spec = ops::Packages::Packages(&packages);

            let (mut examples, mut bins) = (Vec::new(), Vec::new());
            if let Some(ref s) = options.flag_bin {
                bins.push(s.clone());
            }
            if let Some(ref s) = options.flag_example {
                examples.push(s.clone());
            }
            if !bins.is_empty() && !examples.is_empty() {
                return Err(CargoError::from("You can only specify either a --bin or an --example but not both"));
            }

            let pkg = match spec {
                ops::Packages::All => unreachable!("cargo apk supports single package only"),
                ops::Packages::OptOut(_) => unreachable!("cargo apk supports single package only"),
                ops::Packages::Packages(xs) => match xs.len() {
                    0 => workspace.current()?,
                    1 => workspace.members()
                        .find(|pkg| pkg.name() == xs[0])
                        .ok_or_else(|| 
                            CargoError::from(
                                format!("package `{}` is not a member of the workspace", xs[0]))
                        )?,
                    _ => unreachable!("cargo apk supports single package only"),
                }
            };

            if bins.is_empty() && examples.is_empty() {
                bins = pkg.manifest().targets().iter().filter(|a| {
                    !a.is_lib() && !a.is_custom_build() && a.is_bin()
                }).map(|a| a.name().to_owned()).collect();
                if bins.len() >= 2 {
                    return Err(CargoError::from("`cargo apk` can run at most one executable, but \
                        multiple exist"));
                } else if bins.is_empty() {
                    return Err(CargoError::from("a bin target must be available for `cargo apk`"));
                }
            }

            let opts = ops::CompileOptions {
                config: workspace.config(),
                jobs: options.flag_jobs,
                target: Some(build_target),
                features: &options.flag_features,
                all_features: options.flag_all_features,
                no_default_features: options.flag_no_default_features,
                spec: spec,
                mode: ops::CompileMode::Build,
                release: options.flag_release,
                filter: ops::CompileFilter::new(false,
                                            &bins, false,
                                            &[], false,
                                            &examples, false,
                                            &[], false),
                message_format: options.flag_message_format,
                target_rustdoc_args: None,
                target_rustc_args: Some(&extra_args),
            };
            
            ops::compile(workspace, &opts)?;
        }

        // Determine the list of library paths and libraries, and copy them to the right location.
        let shared_objects_to_load = {
            let lib_paths: Vec<String> = {
                if let Ok(f) = File::open(build_target_dir.join("lib_paths")) {
                    let l = BufReader::new(f);
                    l.lines().map(|l| l.unwrap()).collect()
                } else {
                    vec![]
                }
            };

            let libs_list: HashSet<String> = {
                if let Ok(f) = File::open(build_target_dir.join("libs")) {
                    let l = BufReader::new(f);
                    l.lines().map(|l| l.unwrap()).collect()
                } else {
                    HashSet::new()
                }
            };

            let mut shared_objects_to_load = Vec::new();

            for dir in lib_paths.iter() {
                fs::read_dir(&dir).and_then(|paths| {
                    for path in paths {
                        let path = path.unwrap().path();
                        match (path.file_name(), path.extension()) {
                            (Some(filename), Some(ext)) => {
                                let filename = filename.to_str().unwrap();
                                if filename.starts_with("lib") && ext == "so" &&
                                   libs_list.contains(filename)
                                {
                                    shared_objects_to_load.push(filename.to_owned());
                                    fs::copy(&path, native_libraries_dir.join(filename)).unwrap();
                                }
                            }
                            _ => {}
                        }
                    }

                    Ok(())
                }).ok();
            }

            shared_objects_to_load
        };

        abi_libs.insert(abi, shared_objects_to_load);
    }

    // Write the Java source
    build_java_src(workspace, &android_artifacts_dir, &config, &abi_libs)?;

    // Invoking `gradle` from within `android-artifacts` in order to compile the project.
    workspace.config().shell().say("Invoking gradle", 10)?;
    let mut cmd = process(&config.gradle_command);
    if config.release {
        cmd.arg("assembleRelease");
    } else {
        cmd.arg("assembleDebug");
    }
    cmd.cwd(&android_artifacts_dir)
       .exec()?;

    Ok(BuildResult {
        apk_path: {
            let apk_name = if options.flag_release {
                "app/build/outputs/apk/app-release-unsigned.apk"
            } else {
                "app/build/outputs/apk/app-debug.apk"
            };

            android_artifacts_dir.join(apk_name)
        },
    })
}

fn build_android_artifacts_dir(workspace: &Workspace, path: &Path, config: &AndroidConfig) -> Result<(), CargoError> {
    fs::create_dir_all(path.join("app").join("src").join("main")).unwrap();

    {
        fs::create_dir_all(path.join("injected-glue")).unwrap();

        let mut lib = File::create(path.join("injected-glue/lib.rs")).unwrap();
        lib.write_all(&include_bytes!("../../injected-glue/lib.rs")[..]).unwrap();

        let mut ffi = File::create(path.join("injected-glue/ffi.rs")).unwrap();
        ffi.write_all(&include_bytes!("../../injected-glue/ffi.rs")[..]).unwrap();
    }

    build_linker(workspace, path)?;
    build_manifest(workspace, path, config)?;
    build_build_gradle_root(workspace, path, config)?;
    build_build_gradle_proj(workspace, path, config)?;
    build_settings_dot_gradle(workspace, path, config)?;
    build_gradle_properties(workspace, path, config)?;
    build_local_properties(workspace, path, config)?;
    build_assets(workspace, path, config)?;
    build_res(workspace, path, config)?;

    for target in config.build_targets.iter() {
        if fs::metadata(path.join(target)).is_err() {
            fs::DirBuilder::new().recursive(true).create(path.join(target)).unwrap();
        }
    }

    Ok(())
}

fn build_linker(workspace: &Workspace, path: &Path) -> Result<(), CargoError> {
    let exe_file = path.join(if cfg!(target_os = "windows") { "linker_exe.exe" } else { "linker_exe" });

    /*if fs::metadata(&exe_file).is_ok() {
        return;
    }*/

    let mut command = workspace.config().rustc()?.process().arg("-").arg("-o").arg(&exe_file).build_command();

    let mut child = command.stdin(Stdio::piped())
        .spawn()?;
    child.stdin.take().unwrap().write_all(&include_bytes!("../../linker.rs")[..])?;

    let status = child.wait()?;
    assert!(status.success());

    assert!(fs::metadata(&exe_file).is_ok());

    Ok(())
}

fn build_java_src(_: &Workspace, path: &Path, config: &AndroidConfig, abi_libs: &HashMap<&str, Vec<String>>) -> Result<(), CargoError>
{
    let file = path.join("app/src/main/java/rust").join(config.project_name.replace("-", "_"))
                   .join("MainActivity.java");
    fs::create_dir_all(file.parent().unwrap())?;
    //if fs::metadata(&file).is_ok() { return; }
    let mut file = File::create(&file)?;

    let mut libs_string = String::new();

    for (abi, libs) in abi_libs {

        libs_string.push_str(format!("            if (abi.equals(\"{}\")) {{\n",abi).as_str());
        libs_string.push_str(format!("                matched_an_abi = true;\n").as_str());

        for name in libs {
            // Strip off the 'lib' prefix and ".so" suffix.
            let line = format!("                System.loadLibrary(\"{}\");\n",
                name.trim_left_matches("lib").trim_right_matches(".so"));
            libs_string.push_str(&*line);
        }

        libs_string.push_str(format!("                break;\n").as_str());
        libs_string.push_str(format!("            }}\n").as_str());
    }

    write!(file, r#"package rust.{package_name};

import java.lang.UnsupportedOperationException;
import android.os.Build;
import android.util.Log;

public class MainActivity extends android.app.NativeActivity {{

    static {{

        String[] supported_abis;

        try {{
            supported_abis = (String[]) Build.class.getField("SUPPORTED_ABIS").get(null);
        }} catch (Exception e) {{
            // Assume that this is an older phone; use backwards-compatible targets.
            supported_abis = new String[]{{Build.CPU_ABI, Build.CPU_ABI2}};
        }}

        boolean matched_an_abi = false;

        for (String abi : supported_abis) {{
{libs}
        }}

        if (!matched_an_abi) {{
            throw new UnsupportedOperationException("Could not find a native abi target to load");
        }}

    }}
}}"#, libs = libs_string, package_name = config.project_name.replace("-", "_"))?;
    Ok(())
}

fn build_manifest(_: &Workspace, path: &Path, config: &AndroidConfig) -> Result<(), CargoError> {
    fs::create_dir_all(path.join("app/src/main")).unwrap();
    let file = path.join("app/src/main/AndroidManifest.xml");
    //if fs::metadata(&file).is_ok() { return; }
    let mut file = File::create(&file)?;

    // Building application attributes
    let application_attrs = format!(r#"
            android:label="{0}"{1}{2}{3}"#,
        config.package_label,
        config.package_icon.as_ref().map_or(String::new(), |a| format!(r#"
            android:icon="{}""#, a)),
        if config.fullscreen { r#"
            android:theme="@android:style/Theme.DeviceDefault.NoActionBar.Fullscreen""#
        } else { "" },
        config.application_attributes.as_ref().map_or(String::new(), |a| a.replace("\n","\n            "))
    );

    // Buidling activity attributes
    let activity_attrs = format!(r#"
                android:name="rust.{1}.MainActivity"
                android:label="{0}"
                android:configChanges="orientation|keyboardHidden|screenSize" {2}"#,
        config.package_label,
        config.project_name.replace("-", "_"),
        config.activity_attributes.as_ref().map_or(String::new(), |a| a.replace("\n","\n                "))
    );

    // Write final AndroidManifest
    write!(
        file, r#"<?xml version="1.0" encoding="utf-8"?>
<!-- BEGIN_INCLUDE(manifest) -->
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
        package="{package}"
        android:versionCode="1"
        android:versionName="1.0">

    <uses-sdk android:targetSdkVersion="{targetSdkVersion}" android:minSdkVersion="{minSdkVersion}" />

    <uses-feature android:glEsVersion="{glEsVersion}" android:required="true"></uses-feature>
    <uses-permission android:name="android.permission.INTERNET" />
    <uses-permission android:name="android.permission.READ_EXTERNAL_STORAGE" />
    <uses-permission android:name="android.permission.WRITE_EXTERNAL_STORAGE" />

    <application {application_attrs} >
        <activity {activity_attrs} >
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>

</manifest>
<!-- END_INCLUDE(manifest) -->
"#,
        package = config.package_name.replace("-", "_"),
        targetSdkVersion = config.target_sdk_version,
        minSdkVersion = config.min_sdk_version,
        glEsVersion = format!("0x{:04}{:04}", config.opengles_version_major, config.opengles_version_minor),
        application_attrs = application_attrs,
        activity_attrs = activity_attrs
    )?;
    Ok(())
}

fn build_assets(_: &Workspace, path: &Path, config: &AndroidConfig) -> Result<(), CargoError> {
    let src_path = match config.assets_path {
        None => return Ok(()),
        Some(ref p) => p,
    };
    let dst_path = path.join("app").join("src").join("main").join("assets");
    if !dst_path.exists() {
        create_dir_symlink(&src_path, &dst_path)?;
    }
    Ok(())
}

fn build_res(_: &Workspace, path: &Path, config: &AndroidConfig) -> Result<(), CargoError> {
    let src_path = match config.res_path {
        None => return Ok(()),
        Some(ref p) => p,
    };

    if !src_path.exists() {
        return Err(CargoError::from("Resources directory doesn't exist"));
    }

    let dst_path = path.join("app").join("src").join("main").join("res");
    if !dst_path.exists() {
        create_dir_symlink(&src_path, &dst_path)?;
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn create_dir_symlink(src_path: &Path, dst_path: &Path) -> io::Result<()> {
    os::windows::fs::symlink_dir(&src_path, &dst_path)
}

#[cfg(not(target_os = "windows"))]
fn create_dir_symlink(src_path: &Path, dst_path: &Path) -> io::Result<()> {
    os::unix::fs::symlink(&src_path, &dst_path)
}

fn build_build_gradle_root(_: &Workspace, path: &Path, config: &AndroidConfig) -> Result<(), CargoError> {
    let file = path.join("build.gradle");
    //if fs::metadata(&file).is_ok() { return; }
    let mut file = File::create(&file).unwrap();

    write!(file, r#"
buildscript {{
    repositories {{
        jcenter()
    }}
    dependencies {{
        classpath 'com.android.tools.build:gradle:2.3.3'
    }}
}}
allprojects {{
    repositories {{
        jcenter()
    }}
}}
ext {{
    compileSdkVersion = {android_version}
    buildToolsVersion = "{build_tools_version}"
}}
"#, android_version = config.android_version,
    build_tools_version = config.build_tools_version)?;
    Ok(())
}

fn build_build_gradle_proj(_: &Workspace, path: &Path, _config: &AndroidConfig) -> Result<(), CargoError> {
    let file = path.join("app/build.gradle");
    //if fs::metadata(&file).is_ok() { return; }
    let mut file = File::create(&file).unwrap();

    write!(file, r#"
apply plugin: 'com.android.application'

android {{
    compileSdkVersion rootProject.ext.compileSdkVersion
    buildToolsVersion rootProject.ext.buildToolsVersion

    sourceSets {{
        main {{
            jniLibs.srcDirs 'lib/'
        }}
    }}
}}
"#)?;
    Ok(())
}

fn build_settings_dot_gradle(_: &Workspace, path: &Path, _: &AndroidConfig) -> Result<(), CargoError> {
    let file = path.join("settings.gradle");
    //if fs::metadata(&file).is_ok() { return; }
    let mut file = File::create(&file)?;
    write!(file, r"include ':app'")?;
    Ok(())
}

fn build_gradle_properties(_: &Workspace, path: &Path, _: &AndroidConfig) -> Result<(), CargoError> {
    let file = path.join("gradle.properties");
    //if fs::metadata(&file).is_ok() { return; }
    let mut file = File::create(&file)?;
    write!(file, r"android.builder.sdkDownload=false")?;
    Ok(())
}

fn build_local_properties(_: &Workspace, path: &Path, config: &AndroidConfig) -> Result<(), CargoError> {
    let file = path.join("local.properties");
    //if fs::metadata(&file).is_ok() { return; }
    let mut file = File::create(&file)?;

    let sdk_abs_dir = if config.sdk_path.is_absolute() {
        config.sdk_path.clone()
    } else {
        env::current_dir()?.join(&config.sdk_path)
    };

    let ndk_abs_dir = if config.ndk_path.is_absolute() {
        config.ndk_path.clone()
    } else {
        env::current_dir()?.join(&config.ndk_path)
    };

    if cfg!(target_os = "windows") {
        writeln!(file, r"sdk.dir={}", sdk_abs_dir.to_str().unwrap().replace("\\", "\\\\"))?;
    } else {
        writeln!(file, r"sdk.dir={}", sdk_abs_dir.to_str().unwrap())?;
    }

    if cfg!(target_os = "windows") {
        writeln!(file, r"ndk.dir={}", ndk_abs_dir.to_str().unwrap().replace("\\", "\\\\"))?;
    } else {
        writeln!(file, r"ndk.dir={}", ndk_abs_dir.to_str().unwrap())?;
    }

    Ok(())
}
