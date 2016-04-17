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

    // Pick parameters from env vars.
    let gcc = env::var("CARGO_APK_GCC").unwrap();
    let gcc_sysroot = env::var("CARGO_APK_GCC_SYSROOT").unwrap();
    let native_app_glue = env::var("CARGO_APK_NATIVE_APP_GLUE").unwrap();
    let glue_obj = env::var("CARGO_APK_GLUE_OBJ").unwrap();
    let linker_output = env::var("CARGO_APK_LINKER_OUTPUT").unwrap();
    let lib_paths_output = env::var("CARGO_APK_LIB_PATHS_OUTPUT").unwrap();
    let libs_output = env::var("CARGO_APK_LIBS_OUTPUT").unwrap();

    // Write the arguments for the subcommand to pick up.
    {
        let mut lib_paths = File::create(Path::new(&lib_paths_output)).unwrap();
        for lib_path in args.library_path.iter() {
            writeln!(lib_paths, "{}", lib_path.to_string_lossy()).unwrap();
        }

        let mut libs = File::create(Path::new(&libs_output)).unwrap();
        for lib in args.shared_libraries.iter() {
            writeln!(libs, "{}", lib).unwrap();
        }
    }

    // Execute the real linker.
    if Command::new(Path::new(&gcc))
        .args(&*passthrough)
        .arg(native_app_glue)
        .arg(glue_obj)
        .arg("--sysroot").arg(gcc_sysroot)
        .arg("-o").arg(linker_output)
        .arg("-shared")
        .arg("-Wl,-E")
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
}

/// Parses the arguments passed by the CLI and returns two things: the interpretation of some
/// arguments, and a list of other arguments that must be passed through to the real linker.
fn parse_arguments() -> (Args, Vec<String>) {
    let mut result_library_path = Vec::new();
    let mut result_shared_libraries = HashSet::new();
    let mut result_passthrough = Vec::new();

    let args = env::args();
    let mut args = args.skip(1);

    loop {
        let arg = match args.next() {
            Some(arg) => arg,
            None => {
                let args = Args {
                    library_path: result_library_path,
                    shared_libraries: result_shared_libraries,
                };

                return (args, result_passthrough);
            }
        };

        match &*arg {
            "-o" => { args.next(); },
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
