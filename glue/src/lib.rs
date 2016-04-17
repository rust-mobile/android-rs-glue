#![cfg(target_os = "android")]

extern crate cargo_apk_injected_glue;

use std::sync::mpsc::Sender;
use std::os::raw::c_void;

pub use cargo_apk_injected_glue::ffi;

pub use cargo_apk_injected_glue::AssetError;
pub use cargo_apk_injected_glue::Event;
pub use cargo_apk_injected_glue::Motion;
pub use cargo_apk_injected_glue::MotionAction;

/// Return a reference to the application structure.
#[inline]
pub fn get_app<'a>() -> &'a mut ffi::android_app {
    cargo_apk_injected_glue::get_app()
}

/// Adds a sender where events will be sent to.
#[inline]
pub fn add_sender(sender: Sender<Event>) {
    cargo_apk_injected_glue::add_sender(sender)
}

#[inline]
pub fn set_multitouch(multitouch: bool) {
    cargo_apk_injected_glue::set_multitouch(multitouch);
}

/// Adds a sender where events will be sent to, but also sends
/// any missing events to the sender object.
///
/// The missing events happen when the application starts, but before
/// any senders are registered. Since these might be important to certain
/// applications, this function provides that support.
#[inline]
pub fn add_sender_missing(sender: Sender<Event>) {
    cargo_apk_injected_glue::add_sender_missing(sender)
}

/// Returns a handle to the native window.
#[inline]
pub unsafe fn get_native_window() -> *const c_void {
    cargo_apk_injected_glue::get_native_window() as *const _
}

///
#[inline]
pub fn write_log(message: &str) {
    cargo_apk_injected_glue::write_log(message)
}

#[inline]
pub fn load_asset(filename: &str) -> Result<Vec<u8>, AssetError> {
    cargo_apk_injected_glue::load_asset(filename)
}
