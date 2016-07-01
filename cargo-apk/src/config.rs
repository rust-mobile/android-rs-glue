use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use toml;
use toml::Parser as TomlParser;

pub struct Config {
    /// Path to the root of the Android SDK.
    pub sdk_path: PathBuf,
    /// Path to the root of the Android NDK.
    pub ndk_path: PathBuf,
    /// How to invoke `ant`.
    pub ant_command: String,

    /// Name that the package will have on the Android machine.
    /// This is the key that Android uses to identify your package, so it should be unique for
    /// for each application and should contain the vendor's name.
    pub package_name: String,
    /// Name of the project to feed to the SDK. This will be the name of the APK file.
    /// Should be a "system-ish" name, like `my-project`.
    pub project_name: String,
    /// Label for the package.
    pub package_label: String,

    /// List of targets to build the app for. Eg. `arm-linux-androideabi`.
    pub build_targets: Vec<String>,

    /// Version of android for which to compile. TODO: ensure that >=18 because Rustc only supports 18+
    pub android_version: u32,

    /// If `Some`, a path that contains the list of assets to ship as part of the package.
    ///
    /// The assets can later be loaded with the runtime library.
    pub assets_path: Option<PathBuf>,
    /// The external jar path;
    pub jar_libs_path: Option<PathBuf>,

    /// Should we build in release mode?
    pub release: bool,
}

pub fn load(manifest_path: &Path) -> Config {
    // Determine the name of the package and the Android-specific metadata from the Cargo.toml
    let (package_name, manifest_content) = {
        let content = {
            let mut file = File::open(manifest_path).unwrap();
            let mut content = String::new();
            file.read_to_string(&mut content).unwrap();
            content
        };

        let toml = TomlParser::new(&content).parse().unwrap();
        let decoded: TomlPackage = toml::decode(toml["package"].clone()).unwrap();
        let package_name = decoded.name.clone();
        (package_name, decoded.metadata.and_then(|m| m.android))
    };

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

    let jar_libs_path = {
        manifest_content.as_ref().and_then(|a| a.jar_libs_path.as_ref())
            .map(|p| manifest_path.parent().unwrap().join(p))
    };

    // For the moment some fields of the config are dummies.
    Config {
        sdk_path: Path::new(&sdk_path).to_owned(),
        ndk_path: Path::new(&ndk_path).to_owned(),
        ant_command: if cfg!(target_os = "windows") { "ant.bat" } else { "ant" }.to_owned(),
        package_name: manifest_content.as_ref().and_then(|a| a.package_name.clone())
                                       .unwrap_or_else(|| format!("rust.{}", package_name)),
        project_name: package_name.clone(),
        package_label: manifest_content.as_ref().and_then(|a| a.label.clone())
                                       .unwrap_or_else(|| package_name.clone()),
        build_targets: vec!["arm-linux-androideabi".to_owned()],
        android_version: manifest_content.as_ref().and_then(|a| a.android_version).unwrap_or(18),
        assets_path: manifest_content.as_ref().and_then(|a| a.assets.as_ref())
            .map(|p| manifest_path.parent().unwrap().join(p)),
        jar_libs_path: jar_libs_path,
        release: false,
    }
}

#[derive(Debug, Clone, RustcDecodable)]
struct TomlPackage {
    name: String,
    metadata: Option<TomlMetadata>,
}

#[derive(Debug, Clone, RustcDecodable)]
struct TomlMetadata {
    android: Option<TomlAndroid>,
}

#[derive(Debug, Clone, RustcDecodable)]
struct TomlAndroid {
    package_name: Option<String>,
    label: Option<String>,
    assets: Option<String>,
    jar_libs_path: Option<String>,
    android_version: Option<u32>,
}
