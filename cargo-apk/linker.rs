//! This file contains the source code of a dummy linker whose path is going to get passed to
//! rustc. Rustc will think that this program is gcc and pass all the arguments to it. Then this
//! program will tweak the arguments as needed.

use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process;
use std::process::{Command, Stdio};

fn main() {
    let (args, passthrough) = parse_arguments();

    // Write the arguments for the subcommand to pick up.
    {
        let mut lib_paths = File::create(Path::new(&args.cargo_apk_libs_path_output)).unwrap();
        for lib_path in args.library_path.iter() {
            writeln!(lib_paths, "{}", lib_path.to_string_lossy()).unwrap();
        }

        let mut libs = File::create(Path::new(&args.cargo_apk_libs_output)).unwrap();
        for lib in args.shared_libraries.iter() {
            writeln!(libs, "{}", lib).unwrap();
        }
    }

    // Execute the real linker.
    if Command::new(Path::new(&args.cargo_apk_gcc))
        .args(&*passthrough)
        .arg(args.cargo_apk_native_app_glue)
        .arg(args.cargo_apk_glue_obj)
        .arg(args.cargo_apk_glue_lib)
        .arg("-llog").arg("-landroid")      // these two libraries are used by the injected-glue
        .arg("--sysroot").arg(args.cargo_apk_gcc_sysroot)
        .arg("-o").arg(args.cargo_apk_linker_output)
        .arg("-shared")
        .arg("-Wl,-E")
        .arg("-gcc-toolchain").arg(args.cargo_apk_gcc_toolchain)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status().unwrap().code().unwrap() != 0
    {
        println!("Error while executing gcc");
        process::exit(1);
    }
}

struct Args {
    // Paths where to search for libraries as passed with the `-L` options.
    library_path: Vec<PathBuf>,

    // List of libraries to link to as passed with the `-l` option.
    shared_libraries: HashSet<String>,

    cargo_apk_gcc: String,
    cargo_apk_gcc_sysroot: String,
    cargo_apk_gcc_toolchain: String,
    cargo_apk_native_app_glue: String,
    cargo_apk_glue_obj: String,
    cargo_apk_glue_lib: String,
    cargo_apk_linker_output: String,
    cargo_apk_libs_path_output: String,
    cargo_apk_libs_output: String,
}

/// Parses the arguments passed by the CLI and returns two things: the interpretation of some
/// arguments, and a list of other arguments that must be passed through to the real linker.
fn parse_arguments() -> (Args, Vec<String>) {
    let mut result_library_path = Vec::new();
    let mut result_shared_libraries = HashSet::new();
    let mut result_passthrough = Vec::new();

    let mut cargo_apk_gcc: Option<String> = None;
    let mut cargo_apk_gcc_sysroot: Option<String> = None;
    let mut cargo_apk_gcc_toolchain: Option<String> = None;
    let mut cargo_apk_native_app_glue: Option<String> = None;
    let mut cargo_apk_glue_obj: Option<String> = None;
    let mut cargo_apk_glue_lib: Option<String> = None;
    let mut cargo_apk_linker_output: Option<String> = None;
    let mut cargo_apk_libs_path_output: Option<String> = None;
    let mut cargo_apk_libs_output: Option<String> = None;

    let args = env::args();
    let mut args = args.skip(1);

    loop {
        let arg = match args.next() {
            Some(arg) => arg,
            None => {
                let args = Args {
                    library_path: result_library_path,
                    shared_libraries: result_shared_libraries,
                    cargo_apk_gcc: cargo_apk_gcc
                        .expect("Missing cargo_apk_gcc option in linker"),
                    cargo_apk_gcc_sysroot: cargo_apk_gcc_sysroot
                        .expect("Missing cargo_apk_gcc_sysroot option in linker"),
                    cargo_apk_gcc_toolchain: cargo_apk_gcc_toolchain
                        .expect("Missing cargo_apk_gcc_toolchain option in linker"),
                    cargo_apk_native_app_glue: cargo_apk_native_app_glue
                        .expect("Missing cargo_apk_native_app_glue option in linker"),
                    cargo_apk_glue_obj: cargo_apk_glue_obj
                        .expect("Missing cargo_apk_glue_obj option in linker"),
                    cargo_apk_glue_lib: cargo_apk_glue_lib
                        .expect("Missing cargo_apk_glue_lib option in linker"),
                    cargo_apk_linker_output: cargo_apk_linker_output
                        .expect("Missing cargo_apk_linker_output option in linker"),
                    cargo_apk_libs_path_output: cargo_apk_libs_path_output
                        .expect("Missing cargo_apk_libs_path_output option in linker"),
                    cargo_apk_libs_output: cargo_apk_libs_output
                        .expect("Missing cargo_apk_libs_output option in linker"),
                };

                return (args, result_passthrough);
            }
        };

        match &*arg {
            "--cargo-apk-gcc" => {
                cargo_apk_gcc = Some(args.next().unwrap());
            },
            "--cargo-apk-gcc-sysroot" => {
                cargo_apk_gcc_sysroot = Some(args.next().unwrap());
            },
            "--cargo-apk-gcc-toolchain" => {
                cargo_apk_gcc_toolchain = Some(args.next().unwrap());
            }
            "--cargo-apk-native-app-glue" => {
                cargo_apk_native_app_glue = Some(args.next().unwrap());
            },
            "--cargo-apk-glue-obj" => {
                cargo_apk_glue_obj = Some(args.next().unwrap());
            },
            "--cargo-apk-glue-lib" => {
                cargo_apk_glue_lib = Some(args.next().unwrap());
            },
            "--cargo-apk-linker-output" => {
                cargo_apk_linker_output = Some(args.next().unwrap());
            },
            "--cargo-apk-libs-path-output" => {
                cargo_apk_libs_path_output = Some(args.next().unwrap());
            },
            "--cargo-apk-libs-output" => {
                cargo_apk_libs_output = Some(args.next().unwrap());
            },

            "-o" => {
                // Ignore `-o` and the following argument
                args.next();
            },
            "-L" => {
                let path = args.next().expect("-L must be followed by a path");
                result_library_path.push(PathBuf::from(path.clone()));

                // Also pass these through.
                result_passthrough.push(arg);
                result_passthrough.push(path);
            },
            "-l" => {
                let name = args.next().expect("-l must be followed by a library name");
                result_shared_libraries.insert(vec!["lib", &name, ".so"].concat());

                // Also pass these through.
                result_passthrough.push(arg);
                result_passthrough.push(name);
            }
            _ => {
                if arg.starts_with("-l") {
                    result_shared_libraries.insert(vec!["lib", &arg[2..], ".so"].concat());
                }

                // Also pass these through.
                result_passthrough.push(arg);
            }
        };
    }
}
