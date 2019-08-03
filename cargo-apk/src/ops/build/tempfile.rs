use cargo::util::CargoResult;
use std::fs::{self, File};
use std::path::PathBuf;

/// Temporary file implementation that allows creating a file with a specified path which
/// will be deleted when dropped.
pub struct TempFile {
    pub path: PathBuf,
}

impl TempFile {
    /// Create a new `TempFile` using the contents provided by a closure.
    /// If the file already exists, it will be overwritten and then deleted when the instance
    /// is dropped.
    pub fn new<F>(path: PathBuf, write_contents: F) -> CargoResult<TempFile>
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
