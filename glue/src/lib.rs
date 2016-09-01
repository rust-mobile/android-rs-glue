#![cfg(target_os = "android")]

use std::mem;
use std::os::raw::c_void;
use std::sync::mpsc::Sender;

extern {
    fn cargo_apk_injected_glue_get_native_window() -> *const c_void;
    fn cargo_apk_injected_glue_add_sender(sender: *mut c_void);
    fn cargo_apk_injected_glue_add_sender_missing(sender: *mut c_void);
    fn cargo_apk_injected_glue_set_multitouch(multitouch: bool);
    fn cargo_apk_injected_glue_write_log(ptr: *const c_void, len: usize);
    // fn cargo_apk_injected_glue_attach_jvm() ;
    fn cargo_apk_injected_glue_load_asset(ptr: *const c_void, len: usize) -> *mut c_void;
}


//pub use cargo_apk_injected_glue::ffi;
mod touch_event;
pub use touch_event::{TouchEvent, TouchEventType, Pointer, PointerState};

#[derive(Clone, Copy, Debug)]
pub enum KeyEventAction {
    Up,
    Down,
}

/// An event triggered by the Android environment.
#[derive(Copy, Clone, Debug)]
pub enum Event {
    Touch(TouchEvent),
    KeyEvent(KeyEventAction, i32),
    InitWindow,
    SaveState,
    TermWindow,
    GainedFocus,
    LostFocus,
    InputChanged,
    WindowResized,
    WindowRedrawNeeded,
    ContentRectChanged,
    ConfigChanged,
    LowMemory,
    Start,
    Resume,
    Pause,
    Stop,
    Destroy,
}

pub enum AssetError {
    AssetMissing,
    EmptyBuffer,
}

/*/// Return a reference to the application structure.
#[inline]
pub fn get_app<'a>() -> &'a mut ffi::android_app {
    cargo_apk_injected_glue::get_app()
}*/

/// Adds a sender where events will be sent to.
#[inline]
pub fn add_sender(sender: Sender<Event>) {
    unsafe {
        let sender = Box::into_raw(Box::new(sender)) as *mut _;
        cargo_apk_injected_glue_add_sender(sender);
    }
}

#[inline]
pub fn set_multitouch(multitouch: bool) {
    unsafe {
        cargo_apk_injected_glue_set_multitouch(multitouch);
    }
}

/// Adds a sender where events will be sent to, but also sends
/// any missing events to the sender object.
///
/// The missing events happen when the application starts, but before
/// any senders are registered. Since these might be important to certain
/// applications, this function provides that support.
#[inline]
pub fn add_sender_missing(sender: Sender<Event>) {
    unsafe {
        let sender = Box::into_raw(Box::new(sender)) as *mut _;
        cargo_apk_injected_glue_add_sender_missing(sender);
    }
}

/// Returns a handle to the native window.
#[inline]
pub unsafe fn get_native_window() -> *const c_void {
    cargo_apk_injected_glue_get_native_window()
}

///
#[inline]
pub fn write_log(message: &str) {
    unsafe {
        let (message_ptr, message_len) = mem::transmute(message);
        cargo_apk_injected_glue_write_log(message_ptr, message_len);
    }
}

// #[inline]
// pub fn attach_jvm(){
//     unsafe {
//         cargo_apk_injected_glue_attach_jvm()
//     }
// }

#[inline]
pub fn load_asset(filename: &str) -> Result<Vec<u8>, AssetError> {
    unsafe {
        let (filename_ptr, filename_len) = mem::transmute(filename);
        let data = cargo_apk_injected_glue_load_asset(filename_ptr, filename_len);
        let data: Box<Result<Vec<u8>, AssetError>> = Box::from_raw(data as *mut _);
        *data
    }
}

extern crate libc;
mod ffi;
use ffi::{AConfiguration_getDensity, AConfiguration_new, AConfiguration_delete};

#[inline]
pub fn get_screen_density() -> i32{
    unsafe {
        let config = AConfiguration_new();
        let ret = AConfiguration_getDensity(config);
        AConfiguration_delete(config);
        ret
    }
}
