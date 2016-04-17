extern crate rustc_serialize;

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

mod config;

fn main() {
    // Fetching the configuration for the build.
    let config = config::load();

    // Building the `android-artifacts` directory that will contain all the artifacts.
    let android_artifacts_dir = {
        let target_dir = find_project_target();
        target_dir.join("android-artifacts")
    };
    build_android_artifacts_dir(&android_artifacts_dir, &config);

    for build_target in config.build_targets.iter() {
        let build_target_dir = android_artifacts_dir.join(build_target);

        // Finding the tools in the NDK.
        let gcc_path = {
            let arch = if build_target.starts_with("arm") { "arm-linux-androideabi" }
                       else if build_target.starts_with("aarch64") { "aarch64-linux-android" }
                       else if build_target.starts_with("i") { "x86" }
                       else if build_target.starts_with("x86_64") { "x86_64" }
                       else if build_target.starts_with("mipsel") { "mipsel-linux-android" }
                       // TODO: mips64
                       else { panic!("Unknown or incompatible build target: {}", build_target) };

            config.ndk_path.join(format!("toolchains/{}-4.9/prebuilt/linux-x86_64", arch))      // FIXME: correct host arch
                           .join(format!("bin/{}-gcc", arch))
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
        if Command::new(&gcc_path)
            .arg(config.ndk_path.join("sources/android/native_app_glue/android_native_app_glue.c"))
            .arg("-c")
            .arg("-o").arg(build_target_dir.join("android_native_app_glue.o"))
            .arg("--sysroot").arg(&gcc_sysroot)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status().unwrap().code().unwrap() != 0
        {
            exit(1);
        }

        // Compiling injected-glue
        if Command::new("cargo")
            .arg("build")
            .arg("--target").arg(build_target)
            .arg("--manifest-path").arg(android_artifacts_dir.join("injected-glue/Cargo.toml"))
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status().unwrap().code().unwrap() != 0
        {
            exit(1);
        }

        // Compiling glue_obj.rs
        {
            let mut file = File::create(build_target_dir.join("glue_obj.rs")).unwrap();
            file.write_all(&include_bytes!("../glue_obj.rs")[..]).unwrap();
        }
        if Command::new("rustc")
            .arg(build_target_dir.join("glue_obj.rs"))
            .arg("--crate-type").arg("staticlib")
            .arg("--target").arg(build_target)
            .arg("--extern").arg(format!("cargo_apk_injected_glue={}", android_artifacts_dir.join("injected-glue/target").join(build_target).join("debug").join("libcargo_apk_injected_glue.rlib").to_string_lossy()))
            .arg("--emit").arg("obj")
            .arg("-o").arg(build_target_dir.join("glue_obj.o"))
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status().unwrap().code().unwrap() != 0
        {
            exit(1);
        }

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
        if Command::new("cargo").arg("rustc")
            .arg("--verbose")
            .arg("--target").arg(build_target)
            .arg("--")
            .arg("-C").arg(format!("linker={}", android_artifacts_dir.join("linker_exe")
                                                                     .to_string_lossy()))
            .arg("--extern").arg(format!("cargo_apk_injected_glue={}", android_artifacts_dir.join("injected-glue/target").join(build_target).join("debug").join("libcargo_apk_injected_glue.rlib").to_string_lossy()))
            .env("CARGO_APK_GCC", gcc_path.as_os_str())
            .env("CARGO_APK_GCC_SYSROOT", gcc_sysroot.as_os_str())
            .env("CARGO_APK_NATIVE_APP_GLUE", build_target_dir.join("android_native_app_glue.o"))
            .env("CARGO_APK_GLUE_OBJ", build_target_dir.join("glue_obj.o"))
            .env("CARGO_APK_LINKER_OUTPUT", native_libraries_dir.join("libmain.so"))
            .env("CARGO_APK_LIB_PATHS_OUTPUT", build_target_dir.join("lib_paths"))
            .env("CARGO_APK_LIBS_OUTPUT", build_target_dir.join("libs"))
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status().unwrap().code().unwrap() != 0
        {
            exit(1);
        }

        // Determine the list of library paths and libraries, and copy them to the right location.
        {
            let lib_paths: Vec<String> = {
                let l = BufReader::new(File::open(build_target_dir.join("lib_paths")).unwrap());
                l.lines().map(|l| l.unwrap()).collect()
            };

            let libs_list: HashSet<String> = {
                let l = BufReader::new(File::open(build_target_dir.join("libs")).unwrap());
                l.lines().map(|l| l.unwrap()).collect()
            };

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
                                    fs::copy(&path, native_libraries_dir.join(filename)).unwrap();
                                }
                            }
                            _ => {}
                        }
                    }

                    Ok(())
                }).ok();
            }
        }
    }

    // Invoking `ant` from within `android-artifacts` in order to compile the project.
    if Command::new(Path::new("ant"))
        .arg("debug")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .current_dir(android_artifacts_dir.join("build"))
        .status().unwrap().code().unwrap() != 0
    {
        exit(1);
    }
}

// Determines the location of the `target` directory of the project we're compiling.
fn find_project_target() -> PathBuf {
    let output = Command::new("cargo").arg("locate-project").output().unwrap();

    if !output.status.success() {
        if let Some(code) = output.status.code() {
            exit(code);
        } else {
            exit(-1);
        }
    }

    #[derive(RustcDecodable)]
    struct Data { root: String }
    let stdout = String::from_utf8(output.stdout).unwrap();
    let decoded: Data = rustc_serialize::json::decode(&stdout).unwrap();
    let path = Path::new(&decoded.root);
    path.parent().unwrap().join("target")
}

fn build_android_artifacts_dir(path: &Path, config: &config::Config) {
    if fs::metadata(path.join("build")).is_err() {
        fs::DirBuilder::new().recursive(true).create(path.join("build")).unwrap();
    }

    {
        fs::create_dir_all(path.join("injected-glue/src")).unwrap();

        let mut cargo_toml = File::create(path.join("injected-glue/Cargo.toml")).unwrap();
        cargo_toml.write_all(&include_bytes!("../injected-glue/Cargo.toml")[..]).unwrap();

        let mut lib = File::create(path.join("injected-glue/src/lib.rs")).unwrap();
        lib.write_all(&include_bytes!("../injected-glue/src/lib.rs")[..]).unwrap();

        let mut ffi = File::create(path.join("injected-glue/src/ffi.rs")).unwrap();
        ffi.write_all(&include_bytes!("../injected-glue/src/ffi.rs")[..]).unwrap();
    }

    build_linker(path);
    build_manifest(path, "test", "test");
    build_java_src(path);
    build_build_xml(path);
    build_local_properties(path, config);
    build_project_properties(path, config);

    for target in config.build_targets.iter() {
        if fs::metadata(path.join(target)).is_err() {
            fs::DirBuilder::new().recursive(true).create(path.join(target)).unwrap();
        }
    }
}

fn build_linker(path: &Path) {
    let exe_file = path.join("linker_exe");
    let src_file = path.join("linker_src");

    if fs::metadata(&exe_file).is_ok() {
        return;
    }

    {
        let mut src_write = fs::File::create(&src_file).unwrap();
        src_write.write_all(&include_bytes!("../linker.rs")[..]).unwrap();
    }

    let status = Command::new("rustc").arg(src_file).arg("-o").arg(&exe_file).status().unwrap();
    assert!(status.success());

    assert!(fs::metadata(&exe_file).is_ok());
}

fn build_java_src(path: &Path) {
    let file = path.join("build/src/rust/glutin/MainActivity.java");
    if fs::metadata(&file).is_ok() { return; }
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    let mut file = File::create(&file).unwrap();

    let libs_string = "".to_owned();

    // FIXME: this needs to insert each library that we want to load ; the problem is that they could
    //        vary between platforms
    /*for (name, _) in libs.iter() {
        // Strip off the 'lib' prefix and ".so" suffix.
        let line = format!("        System.loadLibrary(\"{}\");\n",
            name.trim_left_matches("lib").trim_right_matches(".so"));
        libs_string.push_str(&*line);
    }*/

    write!(file, r#"package rust.glutin;

public class MainActivity extends android.app.NativeActivity {{
    static {{
        {0}
    }}
}}"#, libs_string).unwrap();
}

fn build_manifest(path: &Path, crate_name: &str, activity_name: &str) {
    let file = path.join("build/AndroidManifest.xml");
    if fs::metadata(&file).is_ok() { return; }
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
        <activity android:name="{1}"
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
"#, crate_name, activity_name).unwrap()
}

fn build_build_xml(path: &Path) {
    let file = path.join("build/build.xml");
    if fs::metadata(&file).is_ok() { return; }
    let mut file = File::create(&file).unwrap();

    write!(file, r#"<?xml version="1.0" encoding="UTF-8"?>
<project name="rust-android" default="help">
    <property file="local.properties" />
    <loadproperties srcFile="project.properties" />
    <import file="custom_rules.xml" optional="true" />
    <import file="${{sdk.dir}}/tools/ant/build.xml" />
</project>
"#).unwrap()
}

fn build_local_properties(path: &Path, config: &config::Config) {
    let file = path.join("build/local.properties");
    if fs::metadata(&file).is_ok() { return; }
    let mut file = File::create(&file).unwrap();

    let abs_dir = if config.sdk_path.is_absolute() {
        config.sdk_path.clone()
    } else {
        env::current_dir().unwrap().join(&config.sdk_path)
    };

    write!(file, r"sdk.dir={}", abs_dir.to_str().unwrap()).unwrap();
}

fn build_project_properties(path: &Path, config: &config::Config) {
    let file = path.join("build/project.properties");
    if fs::metadata(&file).is_ok() { return; }
    let mut file = File::create(&file).unwrap();
    write!(file, r"target=android-{}", config.android_version).unwrap();
}
