#![feature(path, core, io, os, rustc_private)]

extern crate serialize;

use std::collections::HashMap;
use std::old_io::process::Command;
use std::old_io::{File, TempDir};
use std::old_io::fs;

fn main() {
    let (args, passthrough) = parse_arguments();

    // Find all the native shared libraries that exist in the target directory.
    let native_shared_libs = find_native_libs(&args);

    // getting the path from the ANDROID_HOME env
    let sdk_path = std::os::env().into_iter().find(|&(ref k, _)| k.as_slice() == "ANDROID_HOME")
        .map(|(_, v)| Path::new(v)).expect("Please set the ANDROID_HOME environment variable");

    // hardcoding ndk path
    let ndk_path = std::os::env().into_iter().find(|&(ref k, _)| k.as_slice() == "NDK_HOME")
        .map(|(_, v)| Path::new(v)).expect("Please set the NDK_HOME environment variable");

    // hardcoding ndk path
    let standalone_path = std::os::env().into_iter().find(|&(ref k, _)| k.as_slice() == "NDK_STANDALONE")
        .map(|(_, v)| Path::new(v)).unwrap_or(Path::new("/opt/ndk_standalone"));

    // creating the build directory that will contain all the necessary files to create teh apk
    let directory = build_directory(&sdk_path, args.output.filestem_str().unwrap(), &native_shared_libs);

    // Copy the additional native libs into the libs directory.
    for (name, path) in native_shared_libs.iter() {
        fs::copy(path, &directory.path().join("libs").join("armeabi").join(name)).unwrap();
    }

    // compiling android_native_app_glue.c
    if Command::new(standalone_path.join("bin").join("arm-linux-androideabi-gcc"))
        .arg(ndk_path.join("sources").join("android").join("native_app_glue").join("android_native_app_glue.c"))
        .arg("-c")
        .arg("-o").arg(directory.path().join("android_native_app_glue.o"))
        .stdout(std::old_io::process::InheritFd(1)).stderr(std::old_io::process::InheritFd(2))
        .status().unwrap() != std::old_io::process::ExitStatus(0)
    {
        println!("Error while executing gcc");
        std::os::set_exit_status(1);
        return;
    }

    // calling gcc to link to a shared object
    if Command::new(standalone_path.join("bin").join("arm-linux-androideabi-gcc"))
        .args(passthrough.as_slice())
        .arg(directory.path().join("android_native_app_glue.o"))
        .arg("-o").arg(directory.path().join("libs").join("armeabi").join("libmain.so"))
        .arg("-shared")
        .arg("-Wl,-E")
        .stdout(std::old_io::process::InheritFd(1))
        .stderr(std::old_io::process::InheritFd(2))//.cwd(directory.path())
        .status().unwrap() != std::old_io::process::ExitStatus(0)
    {
        println!("Error while executing gcc");
        std::os::set_exit_status(1);
        return;
    }

    // calling objdump to make sure that our object has `ANativeActivity_onCreate`
    // TODO: not working
    /*{
        let mut process =
            Command::new(standalone_path.join("bin").join("arm-linux-androideabi-objdump"))
            .arg("-x").arg(directory.path().join("libs").join("armeabi").join("libmain.so"))
            .stderr(std::old_io::process::InheritFd(2))
            .spawn().unwrap();

        // TODO: use UFCS instead
        fn by_ref<'a, T: Reader>(r: &'a mut T) -> std::old_io::RefReader<'a, T> { r.by_ref() };

        let stdout = process.stdout.as_mut().unwrap();
        let mut stdout = std::old_io::BufferedReader::new(by_ref(stdout));

        if stdout.lines().filter_map(|l| l.ok())
            .find(|line| line.as_slice().contains("ANativeActivity_onCreate")).is_none()
        {
            println!("Error: the output file doesn't contain ANativeActivity_onCreate");
            std::os::set_exit_status(1);
            return;
        }
    }*/

    // executing ant
    if Command::new("ant").arg("debug").stdout(std::old_io::process::InheritFd(1))
        .stderr(std::old_io::process::InheritFd(2)).cwd(directory.path())
        .status().unwrap() != std::old_io::process::ExitStatus(0)
    {
        println!("Error while executing ant debug");
        std::os::set_exit_status(1);
        return;
    }

    // copying apk file to the requested output
    fs::copy(&directory.path().join("bin").join("rust-android-debug.apk"),
        &args.output).unwrap();
}

struct Args {
    output: Path,
}

fn parse_arguments() -> (Args, Vec<String>) {
    let mut result_output = None;
    let mut result_passthrough = Vec::new();

    let args = std::os::args();
    let mut args = args.into_iter().skip(1);

    loop {
        let arg = match args.next() {
            None => return (
                Args {
                    output: result_output.expect("Could not find -o argument")
                },
                result_passthrough
            ),
            Some(arg) => arg
        };

        match arg.as_slice() {
            "-o" => {
                result_output = Some(Path::new(args.next().expect("-o must be followed by the output name")));
            },
            _ => result_passthrough.push(arg)
        };
    }
}

fn find_native_libs(args: &Args) -> HashMap<String, Path> {
    let base_path = args.output.dir_path().join("native");
    let mut native_shared_libs: HashMap<String, Path> = HashMap::new();

    fs::walk_dir(&base_path).and_then(|dirs| {
        for dir in dirs {
            fs::readdir(&dir).and_then(|paths| {
                for path in paths.iter() {
                    match (path.filename_str(), path.extension_str()) {
                        (Some(filename), Some(ext)) => {
                            if filename.starts_with("lib") && ext == "so" {
                                native_shared_libs.insert(filename.to_string(), path.clone());
                            }
                        }
                        _ => {}
                    }
                }
                Ok(())
            }).ok();
        }
        Ok(())
    }).ok();

    native_shared_libs
}

fn build_directory(sdk_dir: &Path, crate_name: &str, libs: &HashMap<String, Path>) -> TempDir {
    use std::old_io::fs;

    let build_directory = TempDir::new("android-rs-glue-rust-to-apk")
        .ok().expect("Could not create temporary build directory");

    let activity_name = if libs.len() > 0 {
        let src_path = build_directory.path().join("src/rust/glutin");
        fs::mkdir_recursive(&src_path, std::old_io::USER_RWX).unwrap();

        File::create(&src_path.join("MainActivity.java")).unwrap()
            .write_str(java_src(libs).as_slice())
            .unwrap();

        "rust.glutin.MainActivity"
    } else {
        "android.app.NativeActivity"
    };

    File::create(&build_directory.path().join("AndroidManifest.xml")).unwrap()
        .write_str(build_manifest(crate_name, activity_name).as_slice())
        .unwrap();

    File::create(&build_directory.path().join("build.xml")).unwrap()
        .write_str(build_build_xml().as_slice())
        .unwrap();

    File::create(&build_directory.path().join("local.properties")).unwrap()
        .write_str(build_local_properties(sdk_dir).as_slice())
        .unwrap();

    File::create(&build_directory.path().join("project.properties")).unwrap()
        .write_str(build_project_properties().as_slice())
        .unwrap();

    {
        let libs_path = build_directory.path().join("libs").join("armeabi");
        fs::mkdir_recursive(&libs_path, std::old_io::USER_RWX).unwrap();
    }

    build_directory
}

fn java_src(libs: &HashMap<String, Path>) -> String {
    let mut libs_string = "".to_string();

    for (name, _) in libs.iter() {
        // Strip off the 'lib' prefix and ".so" suffix. This is safe since libs only get added
        // to the hash map if they start with lib.
        let line = format!("        System.loadLibrary(\"{}\");\n", name.slice(3, name.len()-3));
        libs_string.push_str(line.as_slice());
    }

    format!(r#"package rust.glutin;

public class MainActivity extends android.app.NativeActivity {{
    static {{
        {0}
    }}
}}"#, libs_string)
}

fn build_manifest(crate_name: &str, activity_name: &str) -> String {
    format!(r#"<?xml version="1.0" encoding="utf-8"?>
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
"#, crate_name, activity_name)
}

fn build_build_xml() -> String {
    format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<project name="rust-android" default="help">
    <property file="local.properties" />
    <loadproperties srcFile="project.properties" />
    <import file="custom_rules.xml" optional="true" />
    <import file="${{sdk.dir}}/tools/ant/build.xml" />
</project>
"#)
}

fn build_local_properties(sdk_dir: &Path) -> String {
    use std::os;
    format!(r"sdk.dir={}", os::make_absolute(sdk_dir).unwrap().display())
}

fn build_project_properties() -> String {
    format!(r"target=android-18")
}
