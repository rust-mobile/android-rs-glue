use crate::config::{AndroidBuildTarget, AndroidConfig};
use cargo::core::{Target, TargetKind, Workspace};
use cargo::util::{process, CargoResult, ProcessBuilder};
use failure::format_err;
use std::ffi::OsStr;
use std::path::PathBuf;

/// Returns the directory in which all cargo apk artifacts for the current
/// debug/release configuration should be produced.
pub fn get_root_build_directory(workspace: &Workspace, config: &AndroidConfig) -> PathBuf {
    let android_artifacts_dir = workspace
        .target_dir()
        .join("android-artifacts")
        .into_path_unlocked();

    if config.release {
        android_artifacts_dir.join("release")
    } else {
        android_artifacts_dir.join("debug")
    }
}

/// Returns the sub directory within the root build directory for the specified target.
pub fn get_target_directory(root_build_dir: &PathBuf, target: &Target) -> CargoResult<PathBuf> {
    let target_directory = match target.kind() {
        TargetKind::Bin => root_build_dir.join("bin"),
        TargetKind::ExampleBin => root_build_dir.join("examples"),
        _ => unreachable!("Unexpected target kind"),
    };

    let target_directory = target_directory.join(target.name());
    Ok(target_directory)
}

/// Returns path to NDK provided make
pub fn make_path(config: &AndroidConfig) -> PathBuf {
    config.ndk_path.join("prebuild").join(HOST_TAG).join("make")
}

/// Returns the path to the LLVM toolchain provided by the NDK
pub fn llvm_toolchain_root(config: &AndroidConfig) -> PathBuf {
    config
        .ndk_path
        .join("toolchains")
        .join("llvm")
        .join("prebuilt")
        .join(HOST_TAG)
}

// Helper function for looking for a path based on the platform version
// Calls a closure for each attempt and then return the PathBuf for the first file that exists.
// Uses approach that NDK build tools use which is described at:
// https://developer.android.com/ndk/guides/application_mk
// " - The platform version matching APP_PLATFORM.
//   - The next available API level below APP_PLATFORM. For example, android-19 will be used when
//     APP_PLATFORM is android-20, since there were no new native APIs in android-20.
//   - The minimum API level supported by the NDK."
pub fn find_ndk_path<F>(platform: u32, path_builder: F) -> CargoResult<PathBuf>
where
    F: Fn(u32) -> PathBuf,
{
    let mut tmp_platform = platform;

    // Look for the file which matches the specified platform
    // If that doesn't exist, look for a lower version
    while tmp_platform > 1 {
        let path = path_builder(tmp_platform);
        if path.exists() {
            return Ok(path);
        }

        tmp_platform -= 1;
    }

    // If that doesn't exist... Look for a higher one. This would be the minimum API level supported by the NDK
    tmp_platform = platform;
    while tmp_platform < 100 {
        let path = path_builder(tmp_platform);
        if path.exists() {
            return Ok(path);
        }

        tmp_platform += 1;
    }

    Err(format_err!("Unable to find NDK file"))
}

// Returns path to clang executable/script that should be used to build the target
pub fn find_clang(
    config: &AndroidConfig,
    build_target: AndroidBuildTarget,
) -> CargoResult<PathBuf> {
    let bin_folder = llvm_toolchain_root(config).join("bin");
    find_ndk_path(config.min_sdk_version, |platform| {
        bin_folder.join(format!(
            "{}{}-clang{}",
            build_target.ndk_llvm_triple(),
            platform,
            EXECUTABLE_SUFFIX_CMD
        ))
    })
    .map_err(|_| format_err!("Unable to find NDK clang"))
}

// Returns path to clang++ executable/script that should be used to build the target
pub fn find_clang_cpp(
    config: &AndroidConfig,
    build_target: AndroidBuildTarget,
) -> CargoResult<PathBuf> {
    let bin_folder = llvm_toolchain_root(config).join("bin");
    find_ndk_path(config.min_sdk_version, |platform| {
        bin_folder.join(format!(
            "{}{}-clang++{}",
            build_target.ndk_llvm_triple(),
            platform,
            EXECUTABLE_SUFFIX_CMD
        ))
    })
    .map_err(|_| format_err!("Unable to find NDK clang++"))
}

// Returns path to ar.
pub fn find_ar(config: &AndroidConfig, build_target: AndroidBuildTarget) -> CargoResult<PathBuf> {
    let ar_path = llvm_toolchain_root(config).join("bin").join(format!(
        "{}-ar{}",
        build_target.ndk_triple(),
        EXECUTABLE_SUFFIX_EXE
    ));
    if ar_path.exists() {
        Ok(ar_path)
    } else {
        Err(format_err!(
            "Unable to find ar at `{}`",
            ar_path.to_string_lossy()
        ))
    }
}

// Returns path to readelf
pub fn find_readelf(
    config: &AndroidConfig,
    build_target: AndroidBuildTarget,
) -> CargoResult<PathBuf> {
    let readelf_path = llvm_toolchain_root(config).join("bin").join(format!(
        "{}-readelf{}",
        build_target.ndk_triple(),
        EXECUTABLE_SUFFIX_EXE
    ));
    if readelf_path.exists() {
        Ok(readelf_path)
    } else {
        Err(format_err!(
            "Unable to find readelf at `{}`",
            readelf_path.to_string_lossy()
        ))
    }
}

/// Returns a ProcessBuilder which runs the specified command. Uses "cmd" on windows in order to
/// allow execution of batch files.
pub fn script_process(cmd: impl AsRef<OsStr>) -> ProcessBuilder {
    if cfg!(target_os = "windows") {
        let mut pb = process("cmd");
        pb.arg("/C").arg(cmd);
        pb
    } else {
        process(cmd)
    }
}

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
const HOST_TAG: &str = "windows-x86_64";

#[cfg(all(target_os = "windows", target_pointer_width = "32"))]
const HOST_TAG: &str = "windows";

#[cfg(target_os = "linux")]
const HOST_TAG: &str = "linux-x86_64";

#[cfg(target_os = "macos")]
const HOST_TAG: &str = "darwin-x86_64";

// These are executable suffixes used to simplify building commands.
// On non-windows platforms they are empty.

#[cfg(target_os = "windows")]
const EXECUTABLE_SUFFIX_EXE: &str = ".exe";

#[cfg(not(target_os = "windows"))]
const EXECUTABLE_SUFFIX_EXE: &str = "";

#[cfg(target_os = "windows")]
const EXECUTABLE_SUFFIX_CMD: &str = ".cmd";

#[cfg(not(target_os = "windows"))]
const EXECUTABLE_SUFFIX_CMD: &str = "";

#[cfg(target_os = "windows")]
pub const EXECUTABLE_SUFFIX_BAT: &str = ".bat";

#[cfg(not(target_os = "windows"))]
pub const EXECUTABLE_SUFFIX_BAT: &str = "";
