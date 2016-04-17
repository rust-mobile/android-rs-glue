use std::env;
use std::path::Path;
use std::path::PathBuf;

pub struct Config {
    pub sdk_path: PathBuf,
    pub ndk_path: PathBuf,
    pub build_targets: Vec<String>,
    pub android_version: u32,
}

pub fn load(/*manifest_path: &Path*/) -> Config {
    // For the moment we just build a dummy configuration.

    let ndk_path = env::var("NDK_HOME").expect("Please set the path to the Android NDK with the \
                                                $NDK_HOME environment variable.");

    let sdk_path = {
        let mut try = env::var("ANDROID_SDK_HOME").ok();

        if try.is_none() {
            try = env::var("ANDROID_HOME").ok();
        }

        try.expect("Please set the path to the Android SDK with either the $ANDROID_SDK_HOME or \
                    the $ANDROID_HOME environment variable.")
    };

    Config {
        sdk_path: Path::new(&sdk_path).to_owned(),
        ndk_path: Path::new(&ndk_path).to_owned(),
        build_targets: vec!["arm-linux-androideabi".to_owned()],
        android_version: 23,
    }
}
