#![feature(phase)]

#[phase(plugin)]
extern crate docopt_macros;

extern crate docopt;
extern crate serialize;

use std::io::process::Command;
use std::io::{File, TempDir};

docopt!(Args, "
Usage: rust-to-apk --sdk SDKPATH -o OUTPUT INPUT

Options:
    -h          Print this message
")

fn main() {
    use std::io::fs;

    let args: Args = docopt::FlagParser::parse().unwrap_or_else(|e| e.exit());

    // TODO: check sdk path in ANDROID_HOME if no command line option
    let sdk_path = Path::new(args.arg_SDKPATH);

    let directory = build_directory(&Path::new(args.arg_INPUT), &sdk_path);

    // executing ant
    if Command::new("ant").arg("debug").stdout(std::io::process::InheritFd(1))
        .stderr(std::io::process::InheritFd(2)).cwd(directory.path())
        .status().unwrap() != std::io::process::ExitStatus(0)
    {
        println!("Error while executing ant debug");
        return;
    }

    // copying apk file to OUTPUT
    fs::copy(&directory.path().join("bin").join("rust-android-debug.apk"),
        &Path::new(args.arg_OUTPUT)).unwrap();
}

fn build_directory(library: &Path, sdk_dir: &Path) -> TempDir {
    use std::io::fs;
    use std::os;

    let library = os::make_absolute(library);

    let build_directory = TempDir::new("android-rs-glue-rust-to-apk")
        .expect("Could not create temporary build directory");

    File::create(&build_directory.path().join("AndroidManifest.xml")).unwrap()
        .write_str(build_manifest().as_slice())
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
            .write_str(build_java_class(library.filestem_str().unwrap().slice_from(3)).as_slice())
            .unwrap();
    }

    let libs_path = build_directory.path().join("libs").join("armeabi");
    fs::mkdir_recursive(&libs_path, std::io::UserRWX).unwrap();
    fs::copy(&library, &libs_path.join(library.filename_str().unwrap())).unwrap();

    build_directory
}

fn build_manifest() -> String {
    format!(r#"<?xml version="1.0" encoding="utf-8"?>
<!-- BEGIN_INCLUDE(manifest) -->
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
        package="com.example.native_activity"
        android:versionCode="1"
        android:versionName="1.0">

    <uses-sdk android:minSdkVersion="18" />

    <application android:label="NativeActivity" android:hasCode="true">
        <activity android:name="com.example.native_activity.MyNativeActivity"
                android:label="NativeActivity"
                android:configChanges="orientation|keyboardHidden">
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>

</manifest> 
<!-- END_INCLUDE(manifest) -->
"#)
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

fn build_java_class(libname: &str) -> String {
    format!(r#"
package com.example.native_activity;

import android.app.NativeActivity;
import android.util.Log;

public class MyNativeActivity extends NativeActivity {{
  static {{
    System.loadLibrary("{}");  
  }}

  private static String TAG = "MyNativeActivity";

  public MyNativeActivity() {{
    super();
    Log.v(TAG, "Creating MyNativeActivity");
  }}
}}"#, libname)
}
