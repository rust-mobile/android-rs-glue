pub mod config;
pub mod ops;

pub use config::{AndroidBuildTarget, AndroidConfig};
pub use ops::build::build_apks;
pub use ops::build::compile::{build_shared_libraries, SharedLibraries, SharedLibrary};
pub use ops::build::util::BuildTarget;
