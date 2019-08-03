use crate::config::AndroidConfig;
use cargo::core::compiler::CompileMode;
use cargo::core::compiler::Executor;
use cargo::core::manifest::TargetSourcePath;
use cargo::core::{PackageId, Target, TargetKind, Workspace};
use cargo::util::command_prelude::ArgMatchesExt;
use cargo::util::{CargoResult, ProcessBuilder};
use clap::ArgMatches;
use failure::format_err;
use std::collections::HashSet;
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub type AndroidAbi = String;

/// For each build target and cargo binary or example target, produce a static library which is named based on the cargo target
pub fn build_static_libraries(
    workspace: &Workspace,
    config: &AndroidConfig,
    options: &ArgMatches,
    root_build_dir: &PathBuf,
) -> CargoResult<(HashSet<Target>, Vec<AndroidAbi>)> {
    let injected_glue_src_path = write_injected_glue_src(&root_build_dir)?;

    let mut abis = Vec::new();
    let targets: Arc<Mutex<HashSet<Target>>> = Arc::new(Mutex::new(HashSet::new())); // Set of all example and bin cargo targets built
    for build_target in config.build_targets.iter() {
        // Determine the android ABI
        let abi = get_abi(build_target)?;
        abis.push(abi.to_owned());

        let build_target_dir = root_build_dir.join(abi);

        let injected_glue_lib = build_injected_glue(
            workspace,
            config,
            &injected_glue_src_path,
            &build_target_dir,
            build_target,
        )?;

        // Configure compilation options so that we will build the desired build_target
        let mut opts =
            options.compile_options(workspace.config(), CompileMode::Build, Some(&workspace))?;
        opts.build_config.requested_target = Some((*build_target).clone());

        // Create
        let executor: Arc<dyn Executor> = Arc::new(StaticLibraryExecutor {
            build_target_dir: build_target_dir.clone(),
            injected_glue_lib,
            targets: targets.clone(),
        });

        // Compile all targets for the requested build target
        // Hack to ignore expected error caused by the executor changing the targetkind and other settings.
        // "error: failed to stat ...
        let compilation_result = cargo::ops::compile_with_exec(workspace, &opts, &executor);
        if let Err(err) = &compilation_result {
            let mut output = String::new();
            fmt::write(&mut output, format_args!("{}", err))?;
            if !output.contains(".fingerprint") {
                compilation_result?;
            }
        }
    }

    // Remove the set of targets from the reference counted mutex
    let mut targets = targets.lock().unwrap();
    let targets = std::mem::replace(&mut *targets, HashSet::new());

    Ok((targets, abis))
}

/// Executor which builds binary and example targets as static libraries
struct StaticLibraryExecutor {
    build_target_dir: PathBuf,
    injected_glue_lib: PathBuf,
    targets: Arc<Mutex<HashSet<Target>>>,
}

impl<'a> Executor for StaticLibraryExecutor {
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
            let tmp_file = TempFile::new(tmp_lib_filepath, |lib_src_file| {
                writeln!(
                    lib_src_file,
                    r##"{original_contents}

#[no_mangle]
#[inline(never)]
#[allow(non_snake_case)]
pub extern "C" fn android_main(app: *mut ()) {{
    cargo_apk_injected_glue::android_main2(app as *mut _, move || {{ let _ = main(); }});
}}"##,
                    original_contents = original_contents
                )?;

                Ok(())
            })?;

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
                *source_arg = tmp_file.path.clone().into();
            } else {
                return Err(format_err!(
                    "Unable to replace source argument when buildin target '{}'",
                    target.name()
                ));
            }

            //
            // Change target from bin to staticlib
            //
            for arg in &mut new_args {
                if arg == "bin" {
                    *arg = "staticlib".into();
                }
            }

            //
            // Replace output directory with one inside the build target directory
            //
            let build_path = self.build_target_dir.join("build");
            fs::create_dir_all(&build_path).unwrap();

            let mut iter = new_args.iter_mut().rev().peekable();
            while let Some(arg) = iter.next() {
                if let Some(prev_arg) = iter.peek() {
                    if *prev_arg == "--out-dir" {
                        *arg = build_path.clone().into();
                    }
                }
            }

            // Remove -C extra-filename argument
            {
                let mut extra_filename_index = None;
                for (i, value) in new_args.iter().enumerate() {
                    if value.to_string_lossy().starts_with("extra-filename=") {
                        extra_filename_index = Some(i);
                    }
                }

                if let Some(index) = extra_filename_index {
                    new_args.remove(index - 1);
                    new_args.remove(index - 1);
                }
            }

            //
            // Inject crate dependency for injected glue
            //
            new_args.push("--extern".into());
            let mut arg = OsString::new();
            arg.push("cargo_apk_injected_glue=");
            arg.push(&self.injected_glue_lib);
            new_args.push(arg);

            // Create new command
            let mut cmd = cmd.clone();
            cmd.args_replace(&new_args);

            //
            // Execute the command
            //
            cmd.exec_with_streaming(on_stdout_line, on_stderr_line, false)
                .map(drop)?;

            // Add target to target set
            let mut targets = self.targets.lock().unwrap();

            // Track the cargo targets that are built
            targets.insert(target.clone());
        } else if mode == CompileMode::Test {
            // This occurs when --all-targets is specified
            eprintln!("Ignoring CompileMode::Test for target: {}", target.name());
        } else {
            cmd.exec_with_streaming(on_stdout_line, on_stderr_line, false)
                .map(drop)?
        }

        Ok(())
    }
}

fn write_injected_glue_src(android_artifacts_dir: &Path) -> CargoResult<PathBuf> {
    let injected_glue_path = android_artifacts_dir.join("injected-glue");
    fs::create_dir_all(&injected_glue_path).unwrap();

    let src_path = injected_glue_path.join("lib.rs");
    let mut lib = File::create(&src_path).unwrap();
    lib.write_all(&include_bytes!("../../../injected-glue/lib.rs")[..])
        .unwrap();

    let mut ffi = File::create(injected_glue_path.join("ffi.rs")).unwrap();
    ffi.write_all(&include_bytes!("../../../injected-glue/ffi.rs")[..])
        .unwrap();

    Ok(src_path)
}

fn build_injected_glue(
    workspace: &Workspace,
    config: &AndroidConfig,
    injected_glue_src_path: &PathBuf,
    build_target_dir: &PathBuf,
    build_target: &str,
) -> CargoResult<PathBuf> {
    let rustc = workspace.config().load_global_rustc(Some(&workspace))?;
    let injected_glue_build_path = build_target_dir.join("injected-glue");
    fs::create_dir_all(&injected_glue_build_path)?;

    drop(writeln!(
        workspace.config().shell().err(),
        "Compiling injected-glue for {}",
        build_target
    ));
    let mut cmd = rustc.process();
    cmd.arg(injected_glue_src_path)
        .arg("--edition")
        .arg("2018")
        .arg("--crate-type")
        .arg("rlib");
    if config.release {
        cmd.arg("-C").arg("opt-level=3");
    }
    cmd.arg("--crate-name")
        .arg("cargo_apk_injected_glue")
        .arg("--target")
        .arg(build_target)
        .arg("--out-dir")
        .arg(&injected_glue_build_path);

    cmd.exec()?;

    let stdout = cmd.arg("--print").arg("file-names").exec_with_output()?;
    let stdout = String::from_utf8(stdout.stdout).unwrap();

    Ok(injected_glue_build_path.join(stdout.lines().next().unwrap()))
}

fn get_abi(build_target: &str) -> CargoResult<&str> {
    Ok(if build_target == "armv7-linux-androideabi" {
        "armeabi-v7a"
    } else if build_target == "aarch64-linux-android" {
        "arm64-v8a"
    } else if build_target == "i686-linux-android" {
        "x86"
    } else if build_target == "x86_64-linux-android" {
        "x86_64"
    } else {
        return Err(format_err!(
            "Unknown or incompatible build target: {}",
            build_target
        ));
    })
}

/// Temporary file implementation that allows creating a file with a specified path which
/// will be deleted when dropped.
struct TempFile {
    path: PathBuf,
}

impl TempFile {
    /// Create a new `TempFile` using the contents provided by a closure.
    fn new<F>(path: PathBuf, write_contents: F) -> CargoResult<TempFile>
    where
        F: FnOnce(&mut File) -> CargoResult<()>,
    {
        let tmp_file = TempFile { path };

        // Write the contents to the the temp file
        let mut file = File::create(&tmp_file.path)?;
        write_contents(&mut file)?;

        Ok(tmp_file)
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        // Ignore failure to remove file
        let _ = fs::remove_file(&self.path);
    }
}
