use super::tempfile::TempFile;
use super::util;
use crate::config::AndroidBuildTarget;
use crate::config::AndroidConfig;
use cargo::core::compiler::Executor;
use cargo::core::compiler::{CompileKind, CompileMode, CompileTarget};
use cargo::core::manifest::TargetSourcePath;
use cargo::core::{PackageId, Target, TargetKind, Workspace};
use cargo::util::command_prelude::{ArgMatchesExt, ProfileChecking};
use cargo::util::{process, CargoResult, ProcessBuilder, dylib_path};
use clap::ArgMatches;
use failure::format_err;
use multimap::MultiMap;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::collections::{HashSet, HashMap};

pub struct SharedLibrary {
    pub abi: AndroidBuildTarget,
    pub path: PathBuf,
    pub filename: String,
}

pub struct SharedLibraries {
    pub shared_libraries: MultiMap<Target, SharedLibrary>,
}

/// For each build target and cargo binary or example target, produce a shared library
pub fn build_shared_libraries(
    workspace: &Workspace,
    config: &AndroidConfig,
    options: &ArgMatches,
    root_build_dir: &PathBuf,
) -> CargoResult<SharedLibraries> {
    let android_native_glue_src_path = write_native_app_glue_src(&root_build_dir)?;

    let shared_libraries: Arc<Mutex<MultiMap<Target, SharedLibrary>>> =
        Arc::new(Mutex::new(MultiMap::new()));
    for &build_target in config.build_targets.iter() {
        // Directory that will contain files specific to this build target
        let build_target_dir = root_build_dir.join(build_target.android_abi());
        fs::create_dir_all(&build_target_dir).unwrap();

        // Set environment variables needed for use with the cc crate
        std::env::set_var("CC", util::find_clang(config, build_target)?);
        std::env::set_var("CXX", util::find_clang_cpp(config, build_target)?);
        std::env::set_var("AR", util::find_ar(config, build_target)?);

        // Use libc++. It is current default C++ runtime
        std::env::set_var("CXXSTDLIB", "c++");

        // Generate cmake toolchain and set environment variables to allow projects which use the cmake crate to build correctly
        let cmake_toolchain_path = write_cmake_toolchain(config, &build_target_dir, build_target)?;
        std::env::set_var("CMAKE_TOOLCHAIN_FILE", cmake_toolchain_path);
        std::env::set_var("CMAKE_GENERATOR", r#"Unix Makefiles"#);
        std::env::set_var("CMAKE_MAKE_PROGRAM", util::make_path(config));

        // Build android_native_glue
        let android_native_glue_object = build_android_native_glue(
            config,
            &android_native_glue_src_path,
            &build_target_dir,
            build_target,
        )?;

        // Configure compilation options so that we will build the desired build_target
        let mut opts = options.compile_options(
            workspace.config(),
            CompileMode::Build,
            Some(&workspace),
            ProfileChecking::Unchecked,
        )?;
        opts.build_config.requested_kind =
            CompileKind::Target(CompileTarget::new(build_target.rust_triple())?);

        // Create executor
        let config = Arc::new(config.clone());
        let executor: Arc<dyn Executor> = Arc::new(SharedLibraryExecutor {
            config: Arc::clone(&config),
            build_target_dir: build_target_dir.clone(),
            android_native_glue_object,
            build_target,
            shared_libraries: shared_libraries.clone(),
        });

        // Compile all targets for the requested build target
        cargo::ops::compile_with_exec(workspace, &opts, &executor)?;
    }

    // Remove the set of targets from the reference counted mutex
    let mut shared_libraries = shared_libraries.lock().unwrap();
    let shared_libraries = std::mem::replace(&mut *shared_libraries, MultiMap::new());

    Ok(SharedLibraries { shared_libraries })
}

/// Executor which builds binary and example targets as static libraries
struct SharedLibraryExecutor {
    config: Arc<AndroidConfig>,
    build_target_dir: PathBuf,
    android_native_glue_object: PathBuf,
    build_target: AndroidBuildTarget,

    // Shared libraries built by the executor are added to this multimap
    shared_libraries: Arc<Mutex<MultiMap<Target, SharedLibrary>>>,
}

impl Executor for SharedLibraryExecutor {
    fn exec(
        &self,
        cmd: ProcessBuilder,
        _id: PackageId,
        target: &Target,
        mode: CompileMode,
        on_stdout_line: &mut dyn FnMut(&str) -> CargoResult<()>,
        on_stderr_line: &mut dyn FnMut(&str) -> CargoResult<()>,
    ) -> CargoResult<()> {
        if mode == CompileMode::Build
            && (target.kind() == &TargetKind::Bin || target.kind() == &TargetKind::ExampleBin)
        {
            let mut new_args = cmd.get_args().to_owned();

            //
            // Determine source path
            //
            let path = if let TargetSourcePath::Path(path) = target.src_path() {
                path.to_owned()
            } else {
                // Ignore other values
                return Ok(());
            };

            let original_src_filepath = path.canonicalize()?;

            //
            // Generate source file that will be built
            //
            // Determine the name of the temporary file
            let tmp_lib_filepath = original_src_filepath.parent().unwrap().join(format!(
                "__cargo_apk_{}.tmp",
                original_src_filepath
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(String::new)
            ));

            // Create the temporary file
            let original_contents = fs::read_to_string(original_src_filepath).unwrap();
            let tmp_file = TempFile::new(tmp_lib_filepath.clone(), |lib_src_file| {
                let extra_code = r##"
mod cargo_apk_glue_code {
    use std::os::raw::c_void;

    // Exported function which is called be Android's NativeActivity
    #[no_mangle]
    pub unsafe extern "C" fn ANativeActivity_onCreate(
        activity: *mut c_void,
        saved_state: *mut c_void,
        saved_state_size: usize,
    ) {
        native_app_glue_onCreate(activity, saved_state, saved_state_size);
    }

    extern "C" {
        #[allow(non_snake_case)]
        fn native_app_glue_onCreate(
            activity: *mut c_void,
            saved_state: *mut c_void,
            saved_state_size: usize,
        );
    }

    #[no_mangle]
    extern "C" fn android_main(_app: *mut c_void) {
        let _ = super::main();
    }

    #[link(name = "android")]
    #[link(name = "log")]
    extern "C" {}
}"##;
                writeln!( lib_src_file, "{}\n{}", original_contents, extra_code)?;

                Ok(())
            }).map_err(|e| format_err!(
                "Unable to create temporary source file `{}`. Source directory must be writable. Cargo-apk creates temporary source files as part of the build process. {}.", tmp_lib_filepath.to_string_lossy(), e)
            )?;

            //
            // Replace source argument
            //
            let filename = path.file_name().unwrap().to_owned();
            let source_arg = new_args.iter_mut().find_map(|arg| {
                let path_arg = Path::new(&arg);
                let tmp = path_arg.file_name().unwrap();

                if filename == tmp {
                    Some(arg)
                } else {
                    None
                }
            });

            if let Some(source_arg) = source_arg {
                // Build a new relative path to the temporary source file and use it as the source argument
                // Using an absolute path causes compatibility issues in some cases under windows
                // If a UNC path is used then relative paths used in "include* macros" may not work if
                // the relative path includes "/" instead of "\"
                let path_arg = Path::new(&source_arg);
                let mut path_arg = path_arg.to_path_buf();
                path_arg.set_file_name(tmp_file.path.file_name().unwrap());
                *source_arg = path_arg.into_os_string();
            } else {
                return Err(format_err!(
                    "Unable to replace source argument when building target '{}'",
                    target.name()
                ));
            }

            //
            // Create output directory inside the build target directory
            //
            let build_path = self.build_target_dir.join("build");
            fs::create_dir_all(&build_path).unwrap();

            //
            // Change crate-type from bin to cdylib
            // Replace output directory with the directory we created
            //
            let mut iter = new_args.iter_mut().rev().peekable();
            while let Some(arg) = iter.next() {
                if let Some(prev_arg) = iter.peek() {
                    if *prev_arg == "--crate-type" && arg == "bin" {
                        *arg = "cdylib".into();
                    } else if *prev_arg == "--out-dir" {
                        *arg = build_path.clone().into();
                    }
                }
            }

            // Helper function to build arguments composed of concatenating two strings
            fn build_arg(start: &str, end: impl AsRef<OsStr>) -> OsString {
                let mut new_arg = OsString::new();
                new_arg.push(start);
                new_arg.push(end.as_ref());
                new_arg
            }

            // Determine paths
            let tool_root = util::llvm_toolchain_root(&self.config);
            let linker_path = tool_root
                .join("bin")
                .join(format!("{}-ld", &self.build_target.ndk_triple()));
            let sysroot = tool_root.join("sysroot");
            let version_independent_libraries_path = sysroot
                .join("usr")
                .join("lib")
                .join(&self.build_target.ndk_triple());
            let version_specific_libraries_path =
                util::find_ndk_path(self.config.min_sdk_version, |platform| {
                    version_independent_libraries_path.join(platform.to_string())
                })?;
            let gcc_lib_path = tool_root
                .join("lib/gcc")
                .join(&self.build_target.ndk_triple())
                .join("4.9.x");

            // Add linker arguments
            // Specify linker
            new_args.push(build_arg("-Clinker=", linker_path));

            // Set linker flavor
            new_args.push("-Clinker-flavor=ld".into());

            // Set system root
            new_args.push(build_arg("-Clink-arg=--sysroot=", sysroot));

            // Add version specific libraries directory to search path
            new_args.push(build_arg("-Clink-arg=-L", &version_specific_libraries_path));

            // Add version independent libraries directory to search path
            new_args.push(build_arg(
                "-Clink-arg=-L",
                &version_independent_libraries_path,
            ));

            // Add path to folder containing libgcc.a to search path
            new_args.push(build_arg("-Clink-arg=-L", gcc_lib_path));

            // Add android native glue
            new_args.push(build_arg("-Clink-arg=", &self.android_native_glue_object));

            // Strip symbols for release builds
            if self.config.release {
                new_args.push("-Clink-arg=-strip-all".into());
            }

            // Require position independent code
            new_args.push("-Crelocation-model=pic".into());

            // Create new command
            let mut cmd = cmd.clone();
            cmd.args_replace(&new_args);

            //
            // Execute the command
            //
            cmd.exec_with_streaming(on_stdout_line, on_stderr_line, false)
                .map(drop)?;

            // Execute the command again with the print flag to determine the name of the produced shared library and then add it to the list of shared librares to be added to the APK
            let stdout = cmd.arg("--print").arg("file-names").exec_with_output()?;
            let stdout = String::from_utf8(stdout.stdout).unwrap();
            let library_path = build_path.join(stdout.lines().next().unwrap());

            let mut shared_libraries = self.shared_libraries.lock().unwrap();
            shared_libraries.insert(
                target.clone(),
                SharedLibrary {
                    abi: self.build_target,
                    path: library_path.clone(),
                    filename: format!("lib{}.so", target.name()),
                },
            );

            // If the target uses the C++ standard library, add the appropriate shared library
            // to the list of shared libraries to be added to the APK
            let readelf_path = util::find_readelf(&self.config, self.build_target)?;

            // Gets libraries search paths from compiler
            let mut libs_search_paths = libs_search_paths_from_args(cmd.get_args());

            // Add path for searching version independent libraries like 'libc++_shared.so'
            libs_search_paths.push(version_independent_libraries_path);

            // Add target/ARCH/PROFILE/deps directory for searching dylib/cdylib
            libs_search_paths.push(self.build_target_dir.join("deps"));

            // FIXME: Add extra libraries search paths (from "LD_LIBRARY_PATH")
            libs_search_paths.extend(dylib_path());

            // Find android platform shared libraries
            let android_dylibs = list_android_dylibs(&version_specific_libraries_path)?;

            // The map of [library]: is_processed
            let mut found_dylibs =
                // Add android platform libraries as processed to avoid packaging it
                android_dylibs.into_iter().map(|dylib| (dylib, true))
                .collect::<HashMap<_, _>>();

            // Extract all needed shared libraries from main
            for dylib in list_needed_dylibs(&readelf_path, &library_path)? {
                // Insert new libraries only
                found_dylibs.entry(dylib).or_insert(false);
            }

            while let Some(dylib) = found_dylibs.iter()
                .find(|(_, is_processed)| !*is_processed)
                .map(|(dylib, _)| dylib.clone())
            {
                // Mark library as processed
                *found_dylibs.get_mut(&dylib).unwrap() = true;

                // Find library in known path
                if let Some(path) = find_library_path(&libs_search_paths, &dylib) {
                    // Extract all needed shared libraries recursively
                    for dylib in list_needed_dylibs(&readelf_path, &path)? {
                        // Insert new libraries only
                        found_dylibs.entry(dylib).or_insert(false);
                    }

                    // Add found library
                    shared_libraries.insert(
                        target.clone(),
                        SharedLibrary {
                            abi: self.build_target,
                            path,
                            filename: dylib.clone(),
                        },
                    );
                } else {
                    on_stderr_line(&format!("Warning: Shared library \"{}\" not found.", &dylib))?;
                }
            }
        } else if mode == CompileMode::Test {
            // This occurs when --all-targets is specified
            eprintln!("Ignoring CompileMode::Test for target: {}", target.name());
        } else if mode == CompileMode::Build {
            let mut new_args = cmd.get_args().to_owned();

            //
            // Change crate-type from cdylib to rlib
            //
            let mut iter = new_args.iter_mut().rev().peekable();
            while let Some(arg) = iter.next() {
                if let Some(prev_arg) = iter.peek() {
                    if *prev_arg == "--crate-type" && arg == "cdylib" {
                        *arg = "rlib".into();
                    }
                }
            }

            let mut cmd = cmd.clone();
            cmd.args_replace(&new_args);
            cmd.exec_with_streaming(on_stdout_line, on_stderr_line, false)
                .map(drop)?
        } else {
            cmd.exec_with_streaming(on_stdout_line, on_stderr_line, false)
                .map(drop)?
        }

        Ok(())
    }
}

/// List all linked shared libraries
fn list_needed_dylibs(readelf_path: &Path, library_path: &Path) -> CargoResult<HashSet<String>> {
    let readelf_output = process(readelf_path)
        .arg("-d")
        .arg(&library_path)
        .exec_with_output()?;
    use std::io::BufRead;
    Ok(readelf_output.stdout.lines().filter_map(|l| {
        let l = l.as_ref().unwrap();
        if l.contains("(NEEDED)") {
            if let Some(lib) = l.split("Shared library: [").last() {
                if let Some(lib) = lib.split("]").next() {
                    return Some(lib.into());
                }
            }
        }
        None
    }).collect())
}

/// List Android shared libraries
fn list_android_dylibs(version_specific_libraries_path: &Path) -> CargoResult<HashSet<String>> {
    fs::read_dir(version_specific_libraries_path)?
        .filter_map(|entry| {
            entry.map(|entry| {
                if entry.path().is_file() {
                    if let Some(file_name) = entry.file_name().to_str() {
                        if file_name.ends_with(".so") {
                            return Some(file_name.into());
                        }
                    }
                }
                None
            }).transpose()
        })
        .collect::<Result<_, _>>()
        .map_err(|err| err.into())
}

/// Get native library search paths from rustc args
fn libs_search_paths_from_args(args: &[std::ffi::OsString]) -> Vec<PathBuf> {
    let mut is_search_path = false;
    args.iter().filter_map(|arg| {
        if is_search_path {
            is_search_path = false;
            arg.to_str().and_then(|arg| if arg.starts_with("native=") || arg.starts_with("dependency=") {
                Some(arg.split("=").last().unwrap().into())
            } else {
                None
            })
        } else {
            if arg == "-L" {
                is_search_path = true;
            }
            None
        }
    }).collect()
}

/// Resolves native library using search paths
fn find_library_path<S: AsRef<Path>>(paths: &Vec<PathBuf>, library: S) -> Option<PathBuf> {
    paths.iter().filter_map(|path| {
        let lib_path = path.join(&library);
        if lib_path.is_file() {
            Some(lib_path)
        } else {
            None
        }
    }).nth(0)
}

/// Returns the path to the ".c" file for the android native app glue
fn write_native_app_glue_src(android_artifacts_dir: &Path) -> CargoResult<PathBuf> {
    let output_dir = android_artifacts_dir.join("native_app_glue");
    fs::create_dir_all(&output_dir).unwrap();

    let mut h_file = File::create(output_dir.join("android_native_app_glue.h"))?;
    h_file.write_all(&include_bytes!("../../../native_app_glue/android_native_app_glue.h")[..])?;

    let c_path = output_dir.join("android_native_app_glue.c");
    let mut c_file = File::create(&c_path)?;
    c_file.write_all(&include_bytes!("../../../native_app_glue/android_native_app_glue.c")[..])?;

    Ok(c_path)
}

/// Returns the path to the built object file for the android native glue
fn build_android_native_glue(
    config: &AndroidConfig,
    android_native_glue_src_path: &PathBuf,
    build_target_dir: &PathBuf,
    build_target: AndroidBuildTarget,
) -> CargoResult<PathBuf> {
    let clang = util::find_clang(config, build_target)?;

    let android_native_glue_build_path = build_target_dir.join("android_native_glue");
    fs::create_dir_all(&android_native_glue_build_path)?;
    let android_native_glue_object_path =
        android_native_glue_build_path.join("android_native_glue.o");

    // Will produce warnings when bulding on linux? Create constants for extensions that can be used.. Or have separate functions?
    util::script_process(clang)
        .arg(android_native_glue_src_path)
        .arg("-c")
        .arg("-o")
        .arg(&android_native_glue_object_path)
        .exec()?;

    Ok(android_native_glue_object_path)
}

/// Write a CMake toolchain which will remove references to the rustc build target before including
/// the NDK provided toolchain. The NDK provided android toolchain will set the target appropriately
/// Returns the path to the generated toolchain file
fn write_cmake_toolchain(
    config: &AndroidConfig,
    build_target_dir: &PathBuf,
    build_target: AndroidBuildTarget,
) -> CargoResult<PathBuf> {
    let toolchain_path = build_target_dir.join("cargo-apk.toolchain.cmake");
    let mut toolchain_file = File::create(&toolchain_path).unwrap();
    writeln!(
        toolchain_file,
        r#"set(ANDROID_PLATFORM android-{min_sdk_version})
set(ANDROID_ABI {abi})
string(REPLACE "--target={build_target}" "" CMAKE_C_FLAGS "${{CMAKE_C_FLAGS}}")
string(REPLACE "--target={build_target}" "" CMAKE_CXX_FLAGS "${{CMAKE_CXX_FLAGS}}")
unset(CMAKE_C_COMPILER CACHE)
unset(CMAKE_CXX_COMPILER CACHE)
include("{ndk_path}/build/cmake/android.toolchain.cmake")"#,
        min_sdk_version = config.min_sdk_version,
        ndk_path = config.ndk_path.to_string_lossy().replace("\\", "/"), // Use forward slashes even on windows to avoid path escaping issues.
        build_target = build_target.rust_triple(),
        abi = build_target.android_abi(),
    )?;

    Ok(toolchain_path)
}
