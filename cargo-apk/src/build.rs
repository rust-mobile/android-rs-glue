use std::collections::HashSet;
use std::env;
use std::fs;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::exit;
use std::process::{Command, Stdio};
use termcmd::TermCmd;

use config::Config;

pub struct BuildResult {
    /// The absolute path where the apk is located.
    pub apk_path: PathBuf,
}

pub fn build(manifest_path: &Path, config: &Config) -> BuildResult {
    // First we detect whether `ant` works.
    match Command::new(&config.ant_command).arg("-version").stdout(Stdio::null()).status() {
        Ok(s) if s.success() => (),
        _ => {
            println!("Could not execute `ant`. Did you install it?");
            exit(1);
        }
    }

    // Building the `android-artifacts` directory that will contain all the artifacts.
    let android_artifacts_dir = {
        let target_dir = manifest_path.parent().unwrap().join("target");
        target_dir.join("android-artifacts")
    };
    build_android_artifacts_dir(&android_artifacts_dir, &config);

    for build_target in config.build_targets.iter() {
        let build_target_dir = android_artifacts_dir.join(build_target);

        // Finding the tools in the NDK.
        let gcc_path = {
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

            config.ndk_path.join(format!("toolchains/{}-4.9/prebuilt/{}-x86_64", target_arch, host_os))
                           .join(format!("bin/{}-gcc", target_arch))
        };

        let gcc_sysroot = {
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

        // Compiling android_native_app_glue.c
        TermCmd::new("Compiling android_native_app_glue.c", &gcc_path)
            .arg(config.ndk_path.join("sources/android/native_app_glue/android_native_app_glue.c"))
            .arg("-c")
            .arg("-o").arg(build_target_dir.join("android_native_app_glue.o"))
            .arg("--sysroot").arg(&gcc_sysroot)
            .execute();

        // Compiling injected-glue
        let injected_glue_lib = {
            let mut cmd = TermCmd::new("Compiling injected-glue", "rustc");
            cmd.arg(android_artifacts_dir.join("injected-glue/lib.rs"))
               .arg("--crate-type").arg("rlib")
               .arg("--crate-name").arg("cargo_apk_injected_glue")
               .arg("--target").arg(build_target)
               .arg("--out-dir").arg(&build_target_dir);

            cmd.execute();

            let stdout = cmd.arg("--print").arg("file-names")
                            .exec_stdout();
            let stdout = String::from_utf8(stdout).unwrap();

            build_target_dir.join(stdout.lines().next().unwrap())
        };

        // Compiling glue_obj.rs
        {
            let mut file = File::create(build_target_dir.join("glue_obj.rs")).unwrap();
            file.write_all(&include_bytes!("../glue_obj.rs")[..]).unwrap();
        }
        TermCmd::new("Compiling glue_obj", "rustc")
            .arg(build_target_dir.join("glue_obj.rs"))
            .arg("--crate-type").arg("staticlib")
            .arg("--target").arg(build_target)
            .arg("--extern").arg(format!("cargo_apk_injected_glue={}", injected_glue_lib.to_string_lossy()))
            .arg("--emit").arg("obj")
            .arg("-o").arg(build_target_dir.join("glue_obj.o"))
            .execute();

        // Directory where we will put the native libraries for ant to pick them up.
        let native_libraries_dir = {
            let abi = if build_target.starts_with("arm") { "armeabi" }
                      // TODO: armeabi-v7a
                      else if build_target.starts_with("aarch64") { "arm64-v8a" }
                      else if build_target.starts_with("i") { "x86" }
                      else if build_target.starts_with("x86_64") { "x86_64" }
                      else if build_target.starts_with("mips") { "mips" }
                      // TODO: mips64
                      else { panic!("Unknown or incompatible build target: {}", build_target) };

            android_artifacts_dir.join(format!("build/libs/{}", abi))
        };

        if fs::metadata(&native_libraries_dir).is_err() {
            fs::DirBuilder::new().recursive(true).create(&native_libraries_dir).unwrap();
        }

        // Compiling the crate thanks to `cargo rustc`. We set the linker to `linker_exe`, a hacky
        // linker that will tweak the options passed to `gcc`.
        TermCmd::new("Compiling crate", "cargo").arg("rustc")
            .arg("--target").arg(build_target)
            .arg("--")
            .arg("-C").arg(format!("linker={}", android_artifacts_dir.join(if cfg!(target_os = "windows") { "linker_exe.exe" } else { "linker_exe" })
                                                                     .to_string_lossy()))
            .arg("--extern").arg(format!("cargo_apk_injected_glue={}", injected_glue_lib.to_string_lossy()))
            .inherit_stdout()
            .env("CARGO_APK_GCC", gcc_path.as_os_str())
            .env("CARGO_APK_GCC_SYSROOT", gcc_sysroot.as_os_str())
            .env("CARGO_APK_NATIVE_APP_GLUE", build_target_dir.join("android_native_app_glue.o"))
            .env("CARGO_APK_GLUE_OBJ", build_target_dir.join("glue_obj.o"))
            .env("CARGO_APK_GLUE_LIB", injected_glue_lib)
            .env("CARGO_APK_LINKER_OUTPUT", native_libraries_dir.join("libmain.so"))
            .env("CARGO_APK_LIB_PATHS_OUTPUT", build_target_dir.join("lib_paths"))
            .env("CARGO_APK_LIBS_OUTPUT", build_target_dir.join("libs"))
            .execute();

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

        // Write the Java source
        // FIXME: duh, the file will be replaced every time, so this only works with one target
        build_java_src(&android_artifacts_dir, &config,
                       shared_objects_to_load.iter().map(|s| &**s));
    }

    // Invoking `ant` from within `android-artifacts` in order to compile the project.
    TermCmd::new("Invoking ant", &config.ant_command)
        .arg("debug")
        .current_dir(android_artifacts_dir.join("build"))
        .execute();

    BuildResult {
        apk_path: android_artifacts_dir.join(format!("build/bin/{}-debug.apk", config.project_name)),
    }
}

fn build_android_artifacts_dir(path: &Path, config: &Config) {
    if fs::metadata(path.join("build")).is_err() {
        fs::DirBuilder::new().recursive(true).create(path.join("build")).unwrap();
    }

    {
        fs::create_dir_all(path.join("injected-glue")).unwrap();

        let mut lib = File::create(path.join("injected-glue/lib.rs")).unwrap();
        lib.write_all(&include_bytes!("../injected-glue/lib.rs")[..]).unwrap();

        let mut ffi = File::create(path.join("injected-glue/ffi.rs")).unwrap();
        ffi.write_all(&include_bytes!("../injected-glue/ffi.rs")[..]).unwrap();
    }

    build_linker(path);
    build_manifest(path, config);
    build_build_xml(path, config);
    build_local_properties(path, config);
    build_project_properties(path, config);
    build_assets(path, config);

    for target in config.build_targets.iter() {
        if fs::metadata(path.join(target)).is_err() {
            fs::DirBuilder::new().recursive(true).create(path.join(target)).unwrap();
        }
    }
}

fn build_linker(path: &Path) {
    let exe_file = path.join(if cfg!(target_os = "windows") { "linker_exe.exe" } else { "linker_exe" });
    let src_file = path.join("linker_src");

    /*if fs::metadata(&exe_file).is_ok() {
        return;
    }*/

    {
        let mut src_write = fs::File::create(&src_file).unwrap();
        src_write.write_all(&include_bytes!("../linker.rs")[..]).unwrap();
    }

    let status = Command::new("rustc").arg(src_file).arg("-o").arg(&exe_file).status().unwrap();
    assert!(status.success());

    assert!(fs::metadata(&exe_file).is_ok());
}

fn build_java_src<'a, I>(path: &Path, config: &Config, libs: I)
    where I: Iterator<Item = &'a str>
{
    let file = path.join("build/src/rust").join(config.project_name.replace("-", "_"))
                   .join("MainActivity.java");
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    //if fs::metadata(&file).is_ok() { return; }
    let mut file = File::create(&file).unwrap();

    let mut libs_string = String::new();
    for name in libs {
        // Strip off the 'lib' prefix and ".so" suffix.
        let line = format!("        System.loadLibrary(\"{}\");\n",
            name.trim_left_matches("lib").trim_right_matches(".so"));
        libs_string.push_str(&*line);
    }

    write!(file, r#"package rust.{package_name};

public class MainActivity extends android.app.NativeActivity {{
    static {{
        {libs}
    }}
}}"#, libs = libs_string, package_name = config.project_name.replace("-", "_")).unwrap();
}

fn build_manifest(path: &Path, config: &Config) {
    let file = path.join("build/AndroidManifest.xml");
    //if fs::metadata(&file).is_ok() { return; }
    let mut file = File::create(&file).unwrap();

    write!(file, r#"<?xml version="1.0" encoding="utf-8"?>
<!-- BEGIN_INCLUDE(manifest) -->
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
        package="com.example.native_activity"
        android:versionCode="1"
        android:versionName="1.0">

    <uses-sdk android:minSdkVersion="9" />

    <uses-feature android:glEsVersion="0x00020000" android:required="true"></uses-feature>
    <uses-permission android:name="android.permission.INTERNET" />
    <uses-permission android:name="android.permission.READ_EXTERNAL_STORAGE" />
    <uses-permission android:name="android.permission.WRITE_EXTERNAL_STORAGE" />

    <application android:label="{0}">
        <activity android:name="rust.{1}.MainActivity"
                android:label="{0}"
                android:configChanges="orientation|keyboardHidden">
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>

</manifest>
<!-- END_INCLUDE(manifest) -->
"#, config.package_label, config.project_name.replace("-", "_")).unwrap()
}

fn build_assets(path: &Path, config: &Config) {
    let src_path = match config.assets_path {
        None => return,
        Some(ref p) => p,
    };

    let dst_path = path.join("assets");
    fs::create_dir_all(&dst_path).unwrap();

    fs::hard_link(&src_path, &dst_path).expect("Can not create symlink to assets");
    // TODO: copy files if linking fails
}

fn build_build_xml(path: &Path, config: &Config) {
    let file = path.join("build/build.xml");
    //if fs::metadata(&file).is_ok() { return; }
    let mut file = File::create(&file).unwrap();

    write!(file, r#"<?xml version="1.0" encoding="UTF-8"?>
<project name="{project_name}" default="help">
    <property file="local.properties" />
    <loadproperties srcFile="project.properties" />
    <import file="custom_rules.xml" optional="true" />
    <import file="${{sdk.dir}}/tools/ant/build.xml" />
</project>
"#, project_name = config.project_name).unwrap()
}

fn build_local_properties(path: &Path, config: &Config) {
    let file = path.join("build/local.properties");
    //if fs::metadata(&file).is_ok() { return; }
    let mut file = File::create(&file).unwrap();

    let abs_dir = if config.sdk_path.is_absolute() {
        config.sdk_path.clone()
    } else {
        env::current_dir().unwrap().join(&config.sdk_path)
    };

    if cfg!(target_os = "windows") {
        write!(file, r"sdk.dir={}", abs_dir.to_str().unwrap().replace("\\", "\\\\")).unwrap();
    } else {
        write!(file, r"sdk.dir={}", abs_dir.to_str().unwrap()).unwrap();
    }
}

fn build_project_properties(path: &Path, config: &Config) {
    let file = path.join("build/project.properties");
    //if fs::metadata(&file).is_ok() { return; }
    let mut file = File::create(&file).unwrap();
    write!(file, r"target=android-{}", config.android_version).unwrap();
}
