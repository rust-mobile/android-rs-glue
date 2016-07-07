use std::path::{Path};
use std::io::{Cursor};

#[cfg(not(target_os = "android"))]
pub fn load<P: AsRef<Path>>(path: P) -> Cursor<Vec<u8>> {
    use std::fs::{File};
    use std::io::{Read};

    let mut buf = Vec::new();
    let fullpath = &Path::new("assets").join(&path);
    let mut file = File::open(&fullpath).unwrap();
    file.read_to_end(&mut buf).unwrap();
    Cursor::new(buf)
}

#[cfg(target_os = "android")]
pub fn load<P: AsRef<Path>>(path: P) -> Cursor<Vec<u8>> {
    use android_glue;

    let filename = path.as_ref().to_str()
        .expect("Can`t convert Path to &str");
    match android_glue::load_asset(filename) {
        Ok(buf) => Cursor::new(buf),
        Err(_) => panic!("Can`t load asset '{}'", filename),
    }
}
