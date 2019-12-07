//! Glue for working with cargo-apk
//!
//! Previously, this library provided an abstraction over the event loop.  However, this has been
//! removed in favor of giving users more flexibility.  Use android-ndk or Winit for a higher-level
//! abstraction.

use android_ndk_sys::native_app_glue::android_app;
use std::ptr::NonNull;

extern "C" {
    static ANDROID_APP: *mut android_app;
}

/// Get the `struct android_app` instance from the `android_native_app_glue` code.
pub fn get_android_app() -> NonNull<android_app> {
    NonNull::new(unsafe { ANDROID_APP }).unwrap()
}
