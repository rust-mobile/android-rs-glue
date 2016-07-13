use std::collections::btree_map::BTreeMap;
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

    /// Name of the launcher icon.
    /// Versions of this icon with different resolutions have to reside in the res folder
    pub package_icon: Option<String>,

    /// List of targets to build the app for. Eg. `arm-linux-androideabi`.
    pub build_targets: Vec<String>,

    /// Version of android for which to compile. TODO: ensure that >=18 because Rustc only supports 18+
    pub android_version: u32,

    /// If `Some`, a path that contains the list of assets to ship as part of the package.
    ///
    /// The assets can later be loaded with the runtime library.
    pub assets_path: Option<PathBuf>,

    /// If `Some`, a path that contains the list of resources to ship as part of the package.
    ///
    /// The resources can later be loaded with the runtime library.
    /// This folder contains for example the launcher icon, the styles and resolution dependent images.
    pub res_path: Option<PathBuf>,

    /// Should we build in release mode?
    pub release: bool,

    /// Should this app be in fullscreen mode (hides the title bar)?
    pub fullscreen: bool,

    // Appends this string to the application attributes in the AndroidManifest.xml
    pub application_attributes: Option<String>,

    // Appends this string to the activity attributes in the AndroidManifest.xml
    pub activity_attributes: Option<String>,
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
        package_icon: manifest_content.as_ref().and_then(|a| a.icon.clone()),
        build_targets: vec!["arm-linux-androideabi".to_owned()],
        android_version: manifest_content.as_ref().and_then(|a| a.android_version).unwrap_or(18),
        assets_path: manifest_content.as_ref().and_then(|a| a.assets.as_ref())
            .map(|p| manifest_path.parent().unwrap().join(p)),
        res_path: manifest_content.as_ref().and_then(|a| a.res.as_ref())
            .map(|p| manifest_path.parent().unwrap().join(p)),
        release: false,
        fullscreen: manifest_content.as_ref().and_then(|a| a.fullscreen.clone()).unwrap_or(false),
        application_attributes: manifest_content.as_ref().and_then(|a| map_to_string(a.application_attributes.clone())),
        activity_attributes: manifest_content.as_ref().and_then(|a| map_to_string(a.activity_attributes.clone()))
    }
}

fn map_to_string(input_map: Option<BTreeMap<String, String>>) -> Option<String> {
    // TODO rewrite this in functional style
    if let Some(map) = input_map {
        let mut result = String::new();
        for (key, val) in map {
            result.push_str(&format!("\n{}=\"{}\"", key, val))
        }
        Some(result)
    } else {
        None
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
    icon: Option<String>,
    assets: Option<String>,
    res: Option<String>,
    android_version: Option<u32>,
    fullscreen: Option<bool>,
    application_attributes: Option<BTreeMap<String, String>>,
    activity_attributes: Option<BTreeMap<String, String>>,
}
