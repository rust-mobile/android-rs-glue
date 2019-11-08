use android_ndk::android_app::AndroidApp;
use android_ndk::asset::Asset;
use std::ffi::CString;
use std::io::{BufRead, BufReader};

fn main() {
    let android_app = unsafe { AndroidApp::from_ptr(android_glue::get_android_app()) };

    let f = open_asset(&android_app, "test_asset");
    for line in BufReader::new(f).lines() {
        println!("{:?}", line);
    }
}

fn open_asset(android_app: &AndroidApp, name: &str) -> Asset {
    let asset_manager = android_app.activity().asset_manager();
    asset_manager
        .open(&CString::new(name).unwrap())
        .expect("Could not open asset")
}
