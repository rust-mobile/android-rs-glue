use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::process::{Command, Stdio};

pub struct NdkAccess {
    ndk_path: PathBuf,
}

impl NdkAccess {
    pub fn from_path<P>(path: P) -> NdkAccess where P: Into<PathBuf> {
        let path = path.into();

        // checking that the path is correct
        if !fs::metadata(path.join("ndk-build")).map(|f| f.is_file()).ok().unwrap_or(false) {
            panic!("Incorrect path to NDK");        // TODO: correct error handling
        }

        NdkAccess { ndk_path: path }
    }

    /// Compiles the glue libraries.
    pub fn compile_glue(&self, output: &Path) {
        if Command::new(&toolgccpath.clone())
            .arg(self.ndk_path.join("sources").join("android")
                              .join("native_app_glue").join("android_native_app_glue.c"))
            .arg("-c")
            .arg("-o").arg(output)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status().unwrap().code().unwrap() != 0
        {
            println!("Error while executing gcc");
            process::exit(1);
        }
    }
}
