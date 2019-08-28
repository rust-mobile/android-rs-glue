#![cfg(target_os = "android")]
use std::ffi::c_void;
use std::ptr;

fn main() {
    println!("Android native library test");
    let display = unsafe { eglGetDisplay(ptr::null()) };
    println!("eglGetDisplay(0) result: {:?}", display);
}

// Link to the EGL to ensure that additional libraries can be linked
#[link(name = "EGL")]
extern "C" {
    fn eglGetDisplay(native_display : *const c_void) -> *mut c_void;
}