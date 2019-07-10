use crate::ffi;

extern "C" {
    static mut ANDROID_APP: *mut ffi::android_app;
}

pub fn get_android_app() -> *mut ffi::android_app {
    unsafe { ANDROID_APP }
}
