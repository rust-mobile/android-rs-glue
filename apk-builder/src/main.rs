extern crate serialize;

use std::io::process::Command;
use std::io::{File, TempDir};

fn main() {
    use std::io::fs;

    let (args, passthrough) = parse_arguments();

    // getting the path from the ANDROID_HOME env
    let sdk_path = std::os::env().move_iter().find(|&(ref k, _)| k.as_slice() == "ANDROID_HOME")
        .map(|(_, v)| Path::new(v)).expect("Please set the ANDROID_HOME environment variable");

    // hardcoding ndk path
    let ndk_path = std::os::env().move_iter().find(|&(ref k, _)| k.as_slice() == "NDK_HOME")
        .map(|(_, v)| Path::new(v)).expect("Please set the NDK_HOME environment variable");

    // hardcoding ndk path
    let standalone_path = std::os::env().move_iter().find(|&(ref k, _)| k.as_slice() == "NDK_STANDALONE")
        .map(|(_, v)| Path::new(v)).unwrap_or(Path::new("/opt/ndk_standalone"));

    // creating the build directory that will contain all the necessary files to create teh apk
    let directory = build_directory(&sdk_path, args.output.filestem_str().unwrap());

    // compiling android_native_app_glue.c
    if Command::new(standalone_path.join("bin").join("arm-linux-androideabi-gcc"))
        .arg(ndk_path.join("sources").join("android").join("native_app_glue").join("android_native_app_glue.c"))
        .arg("-c")
        .arg("-o").arg(directory.path().join("android_native_app_glue.o"))
        .stdout(std::io::process::InheritFd(1)).stderr(std::io::process::InheritFd(2))
        .status().unwrap() != std::io::process::ExitStatus(0)
    {
        println!("Error while executing gcc");
        std::os::set_exit_status(1);
        return;
    }
    
    // calling gcc to link to an executable
    if Command::new(standalone_path.join("bin").join("arm-linux-androideabi-gcc"))
        .args(passthrough.as_slice())
        .arg(directory.path().join("android_native_app_glue.o"))
        .arg("-o").arg(directory.path().join("libs").join("armeabi").join("libmain.so"))
        .arg("-Wl,-E")
        .stdout(std::io::process::InheritFd(1))
        .stderr(std::io::process::InheritFd(2))//.cwd(directory.path())
        .status().unwrap() != std::io::process::ExitStatus(0)
    {
        println!("Error while executing gcc");
        std::os::set_exit_status(1);
        return;
    }

    // calling elfedit to turn our executable into a shared object
    if Command::new(standalone_path.join("bin").join("arm-linux-androideabi-elfedit"))
        .arg("--output-type").arg("dyn")
        .arg(directory.path().join("libs").join("armeabi").join("libmain.so"))
        .stdout(std::io::process::InheritFd(1))
        .stderr(std::io::process::InheritFd(2))
        .status().unwrap() != std::io::process::ExitStatus(0)
    {
        println!("Error while executing elfedit");
        std::os::set_exit_status(1);
        return;
    }

    // calling objdump to make sure that our object has `ANativeActivity_onCreate`
    // TODO: not working
    /*{
        let mut process =
            Command::new(standalone_path.join("bin").join("arm-linux-androideabi-objdump"))
            .arg("-x").arg(directory.path().join("libs").join("armeabi").join("libmain.so"))
            .stderr(std::io::process::InheritFd(2))
            .spawn().unwrap();

        // TODO: use UFCS instead
        fn by_ref<'a, T: Reader>(r: &'a mut T) -> std::io::RefReader<'a, T> { r.by_ref() };

        let stdout = process.stdout.as_mut().unwrap();
        let mut stdout = std::io::BufferedReader::new(by_ref(stdout));

        if stdout.lines().filter_map(|l| l.ok())
            .find(|line| line.as_slice().contains("ANativeActivity_onCreate")).is_none()
        {
            println!("Error: the output file doesn't contain ANativeActivity_onCreate");
            std::os::set_exit_status(1);
            return;
        }
    }*/

    // executing ant
    if Command::new("ant").arg("debug").stdout(std::io::process::InheritFd(1))
        .stderr(std::io::process::InheritFd(2)).cwd(directory.path())
        .status().unwrap() != std::io::process::ExitStatus(0)
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
    let mut args = args.move_iter().skip(1);

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

fn build_directory(sdk_dir: &Path, crate_name: &str) -> TempDir {
    use std::io::fs;

    let build_directory = TempDir::new("android-rs-glue-rust-to-apk")
        .ok().expect("Could not create temporary build directory");

    File::create(&build_directory.path().join("AndroidManifest.xml")).unwrap()
        .write_str(build_manifest(crate_name).as_slice())
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
        let src_path = build_directory.path().join("src").join("com").join("example")
            .join("native_activity");
        fs::mkdir_recursive(&src_path, std::io::UserRWX).unwrap();
        File::create(&src_path.join("MyNativeActivity.java")).unwrap()
            .write_str(build_java_class().as_slice())
            .unwrap();
    }

    {
        let libs_path = build_directory.path().join("libs").join("armeabi");
        fs::mkdir_recursive(&libs_path, std::io::UserRWX).unwrap();
    }

    build_directory
}

fn build_manifest(crate_name: &str) -> String {
    format!(r#"<?xml version="1.0" encoding="utf-8"?>
<!-- BEGIN_INCLUDE(manifest) -->
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
        package="com.example.native_activity"
        android:versionCode="1"
        android:versionName="1.0">

    <uses-sdk android:minSdkVersion="9" />

    <application android:label="{0}" android:hasCode="true">
        <activity android:name="com.example.native_activity.MyNativeActivity"
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
"#, crate_name)
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
    format!(r"sdk.dir={}", os::make_absolute(sdk_dir).display())
}

fn build_project_properties() -> String {
    format!(r"target=android-18")
}

fn build_java_class() -> String {
    format!(r#"
package com.example.native_activity;

import android.app.NativeActivity;
import android.util.Log;

public class MyNativeActivity extends NativeActivity {{
  static {{
    System.loadLibrary("main");  
  }}

  private static String TAG = "MyNativeActivity";

  public MyNativeActivity() {{
    super();
    Log.v(TAG, "Creating MyNativeActivity");
  }}
}}"#)
}
