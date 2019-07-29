use cargo::core::{TargetKind, Workspace};
use cargo::ops;
use cargo::util::CargoResult;
use cargo::CliError;
use failure::format_err;
use itertools::Itertools;
use serde::Deserialize;
use std::collections::btree_map::BTreeMap;
use std::env;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::iter::FromIterator;
use std::path::Path;
use std::path::PathBuf;
use toml;

pub struct AndroidConfig {
    /// Name of the cargo package
    pub cargo_package_name: String,

    /// Path to the manifest
    pub manifest_path: PathBuf,
    /// Path to the root of the Android SDK.
    pub sdk_path: PathBuf,
    /// Path to the root of the Android NDK.
    pub ndk_path: PathBuf,

    /// List of targets to build the app for. Eg. `armv7-linux-androideabi`.
    pub build_targets: Vec<String>,

    /// Path to the android.jar for the selected android platform
    pub android_jar_path: PathBuf,

    /// Version of android:targetSdkVersion (optional). Default Value = android_version
    pub target_sdk_version: u32,
    /// Version of android:minSdkVersion (optional). Default Value = android_version
    pub min_sdk_version: u32,

    /// Version of the build tools to use
    pub build_tools_version: String,

    /// Should we build in release mode?
    pub release: bool,

    /// Target configuration settings that are associated with a specific target
    default_target_config: TomlAndroidTarget,

    /// Target specific configuration settings
    target_configs: BTreeMap<(TargetKind, String), TomlAndroidTarget>,
}

impl AndroidConfig {
    /// Builds the android target config based on the default target config and the specific target configs defined in the manifest
    pub fn resolve(&self, target: (TargetKind, String)) -> CargoResult<AndroidTargetConfig> {
        let primary_config = self.target_configs.get(&target);
        let target_name = target.1;
        let is_default_target = target_name == self.cargo_package_name;
        let example = target.0 == TargetKind::ExampleBin;

        Ok(AndroidTargetConfig {
            package_name: primary_config
                .and_then(|a| a.package_name.clone())
                .or_else(|| if is_default_target { self.default_target_config.package_name.clone() } else { None } )
                .unwrap_or_else(|| if example { format!("rust.{}.example.{}", self.cargo_package_name, target_name) } else { format!("rust.{}", target_name) } ),
            package_label: primary_config
                .and_then(|a| a.label.clone())
                .or_else(||  if is_default_target { self.default_target_config.label.clone() } else { None } )
                .unwrap_or_else(|| target_name.clone()),
            package_icon: primary_config
                .and_then(|a| a.icon.clone())
                .or_else(|| self.default_target_config.icon.clone()),
            assets_path: primary_config
                .and_then(|a| a.assets.as_ref())
                .or_else(|| self.default_target_config.assets.as_ref())
                .map(|p| self.manifest_path.parent().unwrap().join(p)),
            res_path: primary_config
                .and_then(|a| a.res.as_ref())
                .or_else(|| self.default_target_config.res.as_ref())
                .map(|p| self.manifest_path.parent().unwrap().join(p)),
            fullscreen: primary_config
                .and_then(|a| a.fullscreen)
                .or_else(|| self.default_target_config.fullscreen)
                .unwrap_or(false),
            application_attributes: primary_config
                .and_then(|a| a.application_attributes.clone())
                .or_else(|| self.default_target_config.application_attributes.clone())
                .map(build_attribute_string),
            activity_attributes: primary_config
                .and_then(|a| a.activity_attributes.clone())
                .or_else(|| self.default_target_config.activity_attributes.clone())
                .map(build_attribute_string),
            opengles_version_major: primary_config
                .and_then(|a| a.opengles_version_major)
                .or_else(|| self.default_target_config.opengles_version_major)
                .unwrap_or(2),
            opengles_version_minor: primary_config
                .and_then(|a| a.opengles_version_minor)
                .or_else(|| self.default_target_config.opengles_version_minor)
                .unwrap_or(0),
            features: primary_config
                .and_then(|a| a.feature.clone())
                .or_else(|| self.default_target_config.feature.clone())
                .unwrap_or_else(Vec::new)
                .into_iter()
                .map(AndroidFeature::from)
                .collect(),
            permissions: primary_config
                .and_then(|a| a.permission.clone())
                .or_else(|| self.default_target_config.permission.clone())
                .unwrap_or_else(Vec::new)
                .into_iter()
                .map(AndroidPermission::from)
                .collect(),
        })
    }
}

#[derive(Clone)]
pub struct AndroidFeature {
    pub name: String,
    pub required: bool,
    pub version: Option<String>,
}

impl From<TomlFeature> for AndroidFeature {
    fn from(f: TomlFeature) -> Self {
        AndroidFeature {
            name: f.name,
            required: f.required.unwrap_or(true),
            version: f.version,
        }
    }
}

#[derive(Clone)]
pub struct AndroidPermission {
    pub name: String,
    pub max_sdk_version: Option<u32>,
}

impl From<TomlPermission> for AndroidPermission {
    fn from(p: TomlPermission) -> Self {
        AndroidPermission {
            name: p.name,
            max_sdk_version: p.max_sdk_version,
        }
    }
}

/// Android build settings for a specific target
pub struct AndroidTargetConfig {
    /// Name that the package will have on the Android machine.
    /// This is the key that Android uses to identify your package, so it should be unique for
    /// for each application and should contain the vendor's name.
    pub package_name: String,

    /// Label for the package.
    pub package_label: String,

    /// Name of the launcher icon.
    /// Versions of this icon with different resolutions have to reside in the res folder
    pub package_icon: Option<String>,

    /// If `Some`, a path that contains the list of assets to ship as part of the package.
    ///
    /// The assets can later be loaded with the runtime library.
    pub assets_path: Option<PathBuf>,

    /// If `Some`, a path that contains the list of resources to ship as part of the package.
    ///
    /// The resources can later be loaded with the runtime library.
    /// This folder contains for example the launcher icon, the styles and resolution dependent images.
    pub res_path: Option<PathBuf>,

    /// Should this app be in fullscreen mode (hides the title bar)?
    pub fullscreen: bool,

    /// Appends this string to the application attributes in the AndroidManifest.xml
    pub application_attributes: Option<String>,

    /// Appends this string to the activity attributes in the AndroidManifest.xml
    pub activity_attributes: Option<String>,

    /// The OpenGL ES major version in the AndroidManifest.xml
    pub opengles_version_major: u8,

    /// The OpenGL ES minor version in the AndroidManifest.xml
    pub opengles_version_minor: u8,

    /// uses-feature in AndroidManifest.xml
    pub features: Vec<AndroidFeature>,

    /// uses-permission in AndroidManifest.xml
    pub permissions: Vec<AndroidPermission>,
}

pub fn load(
    workspace: &Workspace,
    flag_package: &Option<String>,
) -> Result<AndroidConfig, CliError> {
    // Find out the package requested by the user.
    let package = {
        let packages = Vec::from_iter(flag_package.iter().cloned());
        let spec = ops::Packages::Packages(packages);

        match spec {
            ops::Packages::Default => unreachable!("cargo apk supports single package only"),
            ops::Packages::All => unreachable!("cargo apk supports single package only"),
            ops::Packages::OptOut(_) => unreachable!("cargo apk supports single package only"),
            ops::Packages::Packages(xs) => match xs.len() {
                0 => workspace.current()?,
                1 => workspace
                    .members()
                    .find(|pkg| *pkg.name() == xs[0])
                    .ok_or_else(|| {
                        format_err!("package `{}` is not a member of the workspace", xs[0])
                    })?,
                _ => unreachable!("cargo apk supports single package only"),
            },
        }
    };

    // Determine the name of the package and the Android-specific metadata from the Cargo.toml
    let manifest_content = {
        // Load Cargo.toml & parse
        let content = {
            let mut file = File::open(package.manifest_path()).unwrap();
            let mut content = String::new();
            file.read_to_string(&mut content).unwrap();
            content
        };
        let config: TomlConfig = toml::from_str(&content).map_err(failure::Error::from)?;
        config.package.metadata.and_then(|m| m.android)
    };

    // Determine the NDK path
    let ndk_path = env::var("NDK_HOME").map_err(|_| {
        format_err!(
            "Please set the path to the Android NDK with the \
             $NDK_HOME environment variable."
        )
    })?;

    let sdk_path = {
        let mut sdk_path = env::var("ANDROID_SDK_HOME").ok();

        if sdk_path.is_none() {
            sdk_path = env::var("ANDROID_HOME").ok();
        }

        sdk_path.ok_or_else(|| {
            format_err!(
                "Please set the path to the Android SDK with either the $ANDROID_SDK_HOME or \
                 the $ANDROID_HOME environment variable."
            )
        })?
    };

    // Find the highest build tools.
    let build_tools_version = {
        let dir = fs::read_dir(Path::new(&sdk_path).join("build-tools"))
            .map_err(|_| format_err!("Android SDK has no build-tools directory"))?;

        let mut versions = Vec::new();
        for next in dir {
            let next = next.unwrap();

            let meta = next.metadata().unwrap();
            if !meta.is_dir() {
                continue;
            }

            let file_name = next.file_name().into_string().unwrap();
            if !file_name.chars().next().unwrap().is_digit(10) {
                continue;
            }

            versions.push(file_name);
        }

        versions.sort_by(|a, b| b.cmp(&a));
        versions
            .into_iter()
            .next()
            .ok_or_else(|| format_err!("Unable to determine build tools version"))?
    };

    // Determine the Sdk versions (compile, target, min)
    let android_version = manifest_content
        .as_ref()
        .and_then(|a| a.android_version)
        .unwrap_or(29);

    // Check that the tool for the android platform is installed
    let android_jar_path = Path::new(&sdk_path)
        .join("platforms")
        .join(format!("android-{}", android_version))
        .join("android.jar");
    if !android_jar_path.exists() {
        Err(format_err!(
            "'{}' does not exist",
            android_jar_path.to_string_lossy()
        ))?;
    }

    let target_sdk_version = manifest_content
        .as_ref()
        .and_then(|a| a.target_sdk_version)
        .unwrap_or(android_version);
    let min_sdk_version = manifest_content
        .as_ref()
        .and_then(|a| a.min_sdk_version)
        .unwrap_or(18);

    let default_target_config = manifest_content
        .as_ref()
        .map(|a| a.default_target_config.clone())
        .unwrap_or_else(Default::default);

    let mut target_configs = BTreeMap::new();
    manifest_content
        .as_ref()
        .and_then(|a| a.bin.as_ref())
        .unwrap_or(&Vec::new())
        .iter()
        .for_each(|t| {
            target_configs.insert((TargetKind::Bin, t.name.clone()), t.config.clone());
        });
    manifest_content
        .as_ref()
        .and_then(|a| a.example.as_ref())
        .unwrap_or(&Vec::new())
        .iter()
        .for_each(|t| {
            target_configs.insert((TargetKind::ExampleBin, t.name.clone()), t.config.clone());
        });

    // For the moment some fields of the config are dummies.
    Ok(AndroidConfig {
        cargo_package_name: package.name().to_string(),
        manifest_path: package.manifest_path().to_owned(),
        sdk_path: Path::new(&sdk_path).to_owned(),
        ndk_path: Path::new(&ndk_path).to_owned(),
        android_jar_path,
        target_sdk_version,
        min_sdk_version,
        build_tools_version,
        release: false,
        build_targets: manifest_content
            .as_ref()
            .and_then(|a| a.build_targets.clone())
            .unwrap_or_else(|| {
                vec![
                    "armv7-linux-androideabi".to_owned(),
                    "aarch64-linux-android".to_owned(),
                    "i686-linux-android".to_owned(),
                ]
            }),
        default_target_config,
        target_configs,
    })
}

fn build_attribute_string(input_map: BTreeMap<String, String>) -> String {
    input_map
        .iter()
        .map(|(key, val)| format!("\n{}=\"{}\"", key, val))
        .join("")
}

#[derive(Debug, Clone, Deserialize)]
struct TomlConfig {
    package: TomlPackage,
}

#[derive(Debug, Clone, Deserialize)]
struct TomlPackage {
    name: String,
    metadata: Option<TomlMetadata>,
}

#[derive(Debug, Clone, Deserialize)]
struct TomlMetadata {
    android: Option<TomlAndroid>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct TomlAndroid {
    android_version: Option<u32>,
    target_sdk_version: Option<u32>,
    min_sdk_version: Option<u32>,
    build_targets: Option<Vec<String>>,

    #[serde(flatten)]
    default_target_config: TomlAndroidTarget,

    bin: Option<Vec<TomlAndroidSpecificTarget>>,
    example: Option<Vec<TomlAndroidSpecificTarget>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct TomlFeature {
    name: String,
    required: Option<bool>,
    version: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct TomlPermission {
    name: String,
    max_sdk_version: Option<u32>,
}

/// Configuration specific to a single cargo target
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct TomlAndroidSpecificTarget {
    name: String,

    #[serde(flatten)]
    config: TomlAndroidTarget,
}

#[derive(Debug, Default, Clone, Deserialize)]
struct TomlAndroidTarget {
    package_name: Option<String>,
    label: Option<String>,
    icon: Option<String>,
    assets: Option<String>,
    res: Option<String>,
    fullscreen: Option<bool>,
    application_attributes: Option<BTreeMap<String, String>>,
    activity_attributes: Option<BTreeMap<String, String>>,
    opengles_version_major: Option<u8>,
    opengles_version_minor: Option<u8>,
    feature: Option<Vec<TomlFeature>>,
    permission: Option<Vec<TomlPermission>>,
}
