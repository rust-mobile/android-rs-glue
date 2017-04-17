use std::collections::HashMap;
use std::path::PathBuf;
use std::path::Path;
use std::io::Write;
use tempdir::TempDir;

pub struct SdkAccess {
    build_directory: TempDir
}

impl SdkAccess {
    pub fn from_path<P>(sdk_path: P) -> SdkAccess where P: Into<PathBuf> {
        let sdk_path = sdk_path.into();
        let directory = build_directory(&sdk_path, "hello_world", &HashMap::new());

        SdkAccess {
            build_directory: directory,
        }
    }
}

fn build_directory(sdk_dir: &Path, crate_name: &str, libs: &HashMap<String, PathBuf>) -> TempDir {
    let build_directory = TempDir::new("android-rs-glue-rust-to-apk")
                                       .ok().expect("Could not create temporary build directory");

    let activity_name = if libs.len() > 0 {
        let src_path = build_directory.path().join("src/rust/glutin");
        fs::create_dir_all(&src_path).unwrap();

        File::create(&src_path.join("MainActivity.java")).unwrap()
            .write_all(java_src(libs).as_bytes())
            .unwrap();

        "rust.glutin.MainActivity"
    } else {
        "android.app.NativeActivity"
    };

    File::create(&build_directory.path().join("AndroidManifest.xml")).unwrap()
        .write_all(build_manifest(crate_name, activity_name).as_bytes())
        .unwrap();

    File::create(&build_directory.path().join("build.xml")).unwrap()
        .write_all(build_build_xml().as_bytes())
        .unwrap();

    File::create(&build_directory.path().join("local.properties")).unwrap()
        .write_all(build_local_properties(sdk_dir).as_bytes())
        .unwrap();

    File::create(&build_directory.path().join("project.properties")).unwrap()
        .write_all(build_project_properties().as_bytes())
        .unwrap();

    {
        let libs_path = build_directory.path().join("libs").join("armeabi");
        fs::create_dir_all(&libs_path).unwrap();
    }

    {
        // Make sure that 'src' directory is creates
        let src_path = build_directory.path().join("src");
        fs::create_dir_all(&src_path).unwrap();
    }

    build_directory
}

fn java_src(libs: &HashMap<String, PathBuf>) -> String {
    let mut libs_string = "".to_string();

    for (name, _) in libs.iter() {
        // Strip off the 'lib' prefix and ".so" suffix.
        let line = format!("        System.loadLibrary(\"{}\");\n",
            name.trim_left_matches("lib").trim_right_matches(".so"));
        libs_string.push_str(&*line);
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
    let abs_dir = if sdk_dir.is_absolute() {
        sdk_dir.to_path_buf()
    } else {
        env::current_dir().unwrap().join(sdk_dir)
    };
    format!(r"sdk.dir={}", abs_dir.to_str().unwrap())
}

fn build_project_properties() -> String {
    format!(r"target=android-18")
}
