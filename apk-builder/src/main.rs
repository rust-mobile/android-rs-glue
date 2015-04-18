#![feature(convert, path_ext, rustc_private)]

extern crate serialize;
extern crate tempdir;

use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::fs::{File, PathExt};
use std::path::{Path, PathBuf};
use std::process;
use std::process::{Command, Stdio};
use tempdir::{TempDir};

fn main() {
    let (args, passthrough) = parse_arguments();

    // Find all the native shared libraries that exist in the target directory.
    let native_shared_libs = find_native_libs(&args);

    // Get the SDK path from the ANDROID_HOME env.
    let sdk_path = env::var("ANDROID_HOME").ok().expect("Please set the ANDROID_HOME environment variable");
    let sdk_path = Path::new(&sdk_path);

    // Get the NDK path from NDK_HOME env.
    let ndk_path = env::var("NDK_HOME").ok().expect("Please set the NDK_HOME environment variable");
    let ndk_path = Path::new(&ndk_path);

    // Get the standalone NDK path from NDK_STANDALONE env.
    let standalone_path = env::var("NDK_STANDALONE").ok().unwrap_or("/opt/ndk_standalone".to_string());
    let standalone_path = Path::new(&standalone_path);

    // creating the build directory that will contain all the necessary files to create the apk
    let directory = build_directory(&sdk_path, args.output.file_stem().and_then(|s| s.to_str()).unwrap(), &native_shared_libs);

    // Copy the additional native libs into the libs directory.
    for (name, path) in native_shared_libs.iter() {
        fs::copy(path, &directory.path().join("libs").join("armeabi").join(name)).unwrap();
    }

    // Set the paths for the tools used in one central place. Then we also use this to not
    // only invoke the tool when needed, but also to check before first invocation in order
    // to display a nice error message to the user if the tool is missing from the path.
    let toolgccpath = standalone_path.join("bin").join("arm-linux-androideabi-gcc");
    let toolantpath = Path::new("ant");

    if !&toolgccpath.exists() {
        println!("Missing Tool `{}`!", toolgccpath.display());
        process::exit(1);
    }
    // compiling android_native_app_glue.c
    if Command::new(&toolgccpath.clone())
        .arg(ndk_path.join("sources").join("android").join("native_app_glue").join("android_native_app_glue.c"))
        .arg("-c")
        .arg("-o").arg(directory.path().join("android_native_app_glue.o"))
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status().unwrap().code().unwrap() != 0
    {
        println!("Error while executing gcc");
        process::exit(1);
    }

    // calling gcc to link to a shared object
    if Command::new(&toolgccpath.clone())
        .args(passthrough.as_slice())
        .arg(directory.path().join("android_native_app_glue.o"))
        .arg("-o").arg(directory.path().join("libs").join("armeabi").join("libmain.so"))
        .arg("-shared")
        .arg("-Wl,-E")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())//.current_dir(directory.path())
        .status().unwrap().code().unwrap() != 0
    {
        println!("Error while executing gcc");
        process::exit(1);
    }

    // calling objdump to make sure that our object has `ANativeActivity_onCreate`
    // TODO: not working
    /*{
        let mut process =
            Command::new(standalone_path.join("bin").join("arm-linux-androideabi-objdump"))
            .arg("-x").arg(directory.path().join("libs").join("armeabi").join("libmain.so"))
            .stderr(Stdio::inherit())
            .spawn().unwrap();

        // TODO: use UFCS instead
        fn by_ref<'a, T: Reader>(r: &'a mut T) -> std::old_io::RefReader<'a, T> { r.by_ref() };

        let stdout = process.stdout.as_mut().unwrap();
        let mut stdout = std::old_io::BufferedReader::new(by_ref(stdout));

        if stdout.lines().filter_map(|l| l.ok())
            .find(|line| line.as_slice().contains("ANativeActivity_onCreate")).is_none()
        {
            println!("Error: the output file doesn't contain ANativeActivity_onCreate");
            process::exit(1);
        }
    }*/

    copy_assets(&directory.path());

    // executing ant
    let antcmd = Command::new(toolantpath).arg("debug")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .current_dir(directory.path())
        .status();
    if antcmd.is_err() || antcmd.unwrap().code().unwrap() != 0 {
        println!("Error while executing program `ant` debug, or missing program.");
        process::exit(1);
    }

    // copying apk file to the requested output
    fs::copy(&directory.path().join("bin").join("rust-android-debug.apk"),
        &args.output).unwrap();
}

#[cfg(feature = "assets_hack")]
fn copy_assets(build_path: &Path) {
    let cwd = env::current_dir().ok()
        .expect("Can not get current working directory!");
    let assets_path = cwd.join("assets");
    if assets_path.exists() {
        fs::soft_link(&assets_path, &build_path.join("assets"))
            .ok().expect("Can not create symlink to assets");
    }
}

#[cfg(not(feature = "assets_hack"))]
fn copy_assets(_: &Path) {}

struct Args {
    output: PathBuf,
    library_path: Vec<PathBuf>,
    shared_libraries: HashSet<String>,
}

fn parse_arguments() -> (Args, Vec<String>) {
    let mut result_output = None;
    let mut result_library_path = Vec::new();
    let mut result_shared_libraries = HashSet::new();
    let mut result_passthrough = Vec::new();

    let args = env::args();
    let mut args = args.skip(1);

    loop {
        let arg = match args.next() {
            None => return (
                Args {
                    output: result_output.expect("Could not find -o argument"),
                    library_path: result_library_path,
                    shared_libraries: result_shared_libraries,
                },
                result_passthrough
            ),
            Some(arg) => arg
        };

        match arg.as_str() {
            "-o" => {
                result_output = Some(PathBuf::from(args.next().expect("-o must be followed by the output name")));
            },
            "-L" => {
                let path = args.next().expect("-L must be followed by a path");
                result_library_path.push(PathBuf::from(path.clone()));

                // Also pass these through.
                result_passthrough.push(arg);
                result_passthrough.push(path);
            },
            _ => {
                if arg.starts_with("-l") {
                    result_shared_libraries.insert(vec!["lib", &arg[2..], ".so"].concat());
                }
                result_passthrough.push(arg)
            }
        };
    }
}

fn find_native_libs(args: &Args) -> HashMap<String, PathBuf> {
    let mut native_shared_libs: HashMap<String, PathBuf> = HashMap::new();

    for dir in &args.library_path {
        fs::read_dir(&dir).and_then(|paths| {
            for path in paths {
                let path = path.unwrap().path();
                match (path.file_name(), path.extension()) {
                    (Some(filename), Some(ext)) => {
                        let filename = filename.to_str().unwrap();
                        if filename.starts_with("lib")
                            && ext == "so"
                            && args.shared_libraries.contains(filename) {
                            native_shared_libs.insert(filename.to_string(), path.clone());
                        }
                    }
                    _ => {}
                }
            }
            Ok(())
        }).ok();
    }
    native_shared_libs
}

fn build_directory(sdk_dir: &Path, crate_name: &str, libs: &HashMap<String, PathBuf>) -> TempDir {
    use std::io::Write;

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
        // Strip off the 'lib' prefix and ".so" suffix. This is safe since libs only get added
        // to the hash map if they start with lib.
        let line = format!("        System.loadLibrary(\"{}\");\n", &name[3..]);
        libs_string.push_str(line.as_str());
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
