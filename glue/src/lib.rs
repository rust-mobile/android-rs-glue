#![cfg(target_os = "android")]

extern {
    fn cargo_apk_injected_glue_get_native_window() -> *const c_void;
    fn cargo_apk_injected_glue_add_sender(sender: *mut ());
    fn cargo_apk_injected_glue_add_sender_missing(sender: *mut ());
    fn cargo_apk_injected_glue_set_multitouch(multitouch: bool);
    fn cargo_apk_injected_glue_write_log(ptr: *const (), len: usize);
}

use std::mem;
use std::os::raw::c_void;
use std::sync::mpsc::Sender;

//pub use cargo_apk_injected_glue::ffi;

/// An event triggered by the Android environment.
#[derive(Clone, Copy, Debug)]
pub enum Event {
    EventMotion(Motion),
    EventKeyUp,
    EventKeyDown,
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

/// Data about a motion event.
#[derive(Clone, Copy, Debug)]
pub struct Motion {
    pub action: MotionAction,
    pub pointer_id: i32,
    pub x: f32,
    pub y: f32,
}

/// The type of pointer action in a motion event.
#[derive(Clone, Copy, Debug)]
pub enum MotionAction {
    Down,
    Move,
    Up,
    Cancel,
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

#[inline]
pub fn load_asset(filename: &str) -> Result<Vec<u8>, AssetError> {
    unimplemented!()    // FIXME: !
    //cargo_apk_injected_glue::load_asset(filename)
}
