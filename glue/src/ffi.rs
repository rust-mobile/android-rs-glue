#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use libc;

/*
 * android_native_app_glue.h
 */
#[repr(C)]
pub struct android_app {
    pub userData: *mut libc::c_void,
    pub onAppCmd: extern fn(*mut android_app, libc::int32_t),
    pub onInputEvent: extern fn(*mut android_app, *const AInputEvent) -> libc::int32_t,
    pub activity: *const ANativeActivity,
    pub config: *const AConfiguration,
    pub savedState: *mut libc::c_void,
    pub savedStateSize: libc::size_t,
    pub looper: *const ALooper,
    pub inputQueue: *const AInputQueue,
    pub window: *const ANativeWindow,
    pub contentRect: ARect,
    pub activityState: libc::c_int,
    pub destroyRequested: libc::c_int,
}

#[repr(C)]
pub struct android_poll_source {
    pub id: libc::int32_t,      // can be LOOPER_ID_MAIN, LOOPER_ID_INPUT or LOOPER_ID_USER
    pub app: *mut android_app,
    pub process: extern fn(*mut android_app, *mut android_poll_source),
}

pub static LOOPER_ID_MAIN: libc::int32_t = 1;
pub static LOOPER_ID_INPUT: libc::int32_t = 1;
pub static LOOPER_ID_USER: libc::int32_t = 1;

pub static APP_CMD_INPUT_CHANGED: libc::int32_t = 0;
pub static APP_CMD_INIT_WINDOW: libc::int32_t = 1;
pub static APP_CMD_TERM_WINDOW: libc::int32_t = 2;
pub static APP_CMD_WINDOW_RESIZED: libc::int32_t = 3;
pub static APP_CMD_WINDOW_REDRAW_NEEDED: libc::int32_t = 4;
pub static APP_CMD_CONTENT_RECT_CHANGED: libc::int32_t = 5;
pub static APP_CMD_GAINED_FOCUS: libc::int32_t = 6;
pub static APP_CMD_LOST_FOCUS: libc::int32_t = 7;
pub static APP_CMD_CONFIG_CHANGED: libc::int32_t = 8;
pub static APP_CMD_LOW_MEMORY: libc::int32_t = 9;
pub static APP_CMD_START: libc::int32_t = 10;
pub static APP_CMD_RESUME: libc::int32_t = 11;
pub static APP_CMD_SAVE_STATE: libc::int32_t = 12;
pub static APP_CMD_PAUSE: libc::int32_t = 13;
pub static APP_CMD_STOP: libc::int32_t = 14;
pub static APP_CMD_DESTROY: libc::int32_t = 15;

extern {
    pub fn app_dummy();
}


/*
 * asset_manager.h
 */
pub type AAssetManager = ();


/*
 * configuration.h
 */
pub type AConfiguration = ();


/*
 * input.h
 */
pub type AInputQueue = ();
pub type AInputEvent = ();

pub static AKEY_STATE_UNKNOWN: libc::int32_t = -1;
pub static AKEY_STATE_UP: libc::int32_t = 0;
pub static AKEY_STATE_DOWN: libc::int32_t = 1;
pub static AKEY_STATE_VIRTUAL: libc::int32_t = 2;

pub static AMETA_NONE: libc::int32_t = 0;
pub static AMETA_ALT_ON: libc::int32_t = 0x02;
pub static AMETA_ALT_LEFT_ON: libc::int32_t = 0x10;
pub static AMETA_ALT_RIGHT_ON: libc::int32_t = 0x20;
pub static AMETA_SHIFT_ON: libc::int32_t = 0x01;
pub static AMETA_SHIFT_LEFT_ON: libc::int32_t = 0x40;
pub static AMETA_SHIFT_RIGHT_ON: libc::int32_t = 0x80;
pub static AMETA_SYM_ON: libc::int32_t = 0x04;
pub static AMETA_FUNCTION_ON: libc::int32_t = 0x08;
pub static AMETA_CTRL_ON: libc::int32_t = 0x1000;
pub static AMETA_CTRL_LEFT_ON: libc::int32_t = 0x2000;
pub static AMETA_CTRL_RIGHT_ON: libc::int32_t = 0x4000;
pub static AMETA_META_ON: libc::int32_t = 0x10000;
pub static AMETA_META_LEFT_ON: libc::int32_t = 0x20000;
pub static AMETA_META_RIGHT_ON: libc::int32_t = 0x40000;
pub static AMETA_CAPS_LOCK_ON: libc::int32_t = 0x100000;
pub static AMETA_NUM_LOCK_ON: libc::int32_t = 0x200000;
pub static AMETA_SCROLL_LOCK_ON: libc::int32_t = 0x400000;

pub static AINPUT_EVENT_TYPE_KEY: libc::int32_t = 1;
pub static AINPUT_EVENT_TYPE_MOTION: libc::int32_t = 2;

pub static AKEY_EVENT_ACTION_DOWN: libc::int32_t = 0;
pub static AKEY_EVENT_ACTION_UP: libc::int32_t = 1;
pub static AKEY_EVENT_ACTION_MULTIPLE: libc::int32_t = 2;

pub static AKEY_EVENT_FLAG_WOKE_HERE: libc::int32_t = 0x1;
pub static AKEY_EVENT_FLAG_SOFT_KEYBOARD: libc::int32_t = 0x2;
pub static AKEY_EVENT_FLAG_KEEP_TOUCH_MODE: libc::int32_t = 0x4;
pub static AKEY_EVENT_FLAG_FROM_SYSTEM: libc::int32_t = 0x8;
pub static AKEY_EVENT_FLAG_EDITOR_ACTION: libc::int32_t = 0x10;
pub static AKEY_EVENT_FLAG_CANCELED: libc::int32_t = 0x20;
pub static AKEY_EVENT_FLAG_VIRTUAL_HARD_KEY: libc::int32_t = 0x40;
pub static AKEY_EVENT_FLAG_LONG_PRESS: libc::int32_t = 0x80;
pub static AKEY_EVENT_FLAG_CANCELED_LONG_PRESS: libc::int32_t = 0x100;
pub static AKEY_EVENT_FLAG_TRACKING: libc::int32_t = 0x200;
pub static AKEY_EVENT_FLAG_FALLBACK: libc::int32_t = 0x400;

pub static AMOTION_EVENT_ACTION_POINTER_INDEX_SHIFT: libc::int32_t = 8;

pub static AMOTION_EVENT_ACTION_MASK: libc::int32_t = 0xff;
pub static AMOTION_EVENT_ACTION_POINTER_INDEX_MASK: libc::int32_t = 0xff00;
pub static AMOTION_EVENT_ACTION_DOWN: libc::int32_t = 0;
pub static AMOTION_EVENT_ACTION_UP: libc::int32_t = 1;
pub static AMOTION_EVENT_ACTION_MOVE: libc::int32_t = 2;
pub static AMOTION_EVENT_ACTION_CANCEL: libc::int32_t = 3;
pub static AMOTION_EVENT_ACTION_OUTSIDE: libc::int32_t = 4;
pub static AMOTION_EVENT_ACTION_POINTER_DOWN: libc::int32_t = 5;
pub static AMOTION_EVENT_ACTION_POINTER_UP: libc::int32_t = 6;
pub static AMOTION_EVENT_ACTION_HOVER_MOVE: libc::int32_t = 7;
pub static AMOTION_EVENT_ACTION_SCROLL: libc::int32_t = 8;
pub static AMOTION_EVENT_ACTION_HOVER_ENTER: libc::int32_t = 9;
pub static AMOTION_EVENT_ACTION_HOVER_EXIT: libc::int32_t = 10;

pub static AMOTION_EVENT_FLAG_WINDOW_IS_OBSCURED: libc::int32_t = 0x1;

pub static AMOTION_EVENT_EDGE_FLAG_NONE: libc::int32_t = 0;
pub static AMOTION_EVENT_EDGE_FLAG_TOP: libc::int32_t = 0x01;
pub static AMOTION_EVENT_EDGE_FLAG_BOTTOM: libc::int32_t = 0x02;
pub static AMOTION_EVENT_EDGE_FLAG_LEFT: libc::int32_t = 0x04;
pub static AMOTION_EVENT_EDGE_FLAG_RIGHT: libc::int32_t = 0x08;

pub static AMOTION_EVENT_AXIS_X: libc::int32_t = 0;
pub static AMOTION_EVENT_AXIS_Y: libc::int32_t = 1;
pub static AMOTION_EVENT_AXIS_PRESSURE: libc::int32_t = 2;
pub static AMOTION_EVENT_AXIS_SIZE: libc::int32_t = 3;
pub static AMOTION_EVENT_AXIS_TOUCH_MAJOR: libc::int32_t = 4;
pub static AMOTION_EVENT_AXIS_TOUCH_MINOR: libc::int32_t = 5;
pub static AMOTION_EVENT_AXIS_TOOL_MAJOR: libc::int32_t = 6;
pub static AMOTION_EVENT_AXIS_TOOL_MINOR: libc::int32_t = 7;
pub static AMOTION_EVENT_AXIS_ORIENTATION: libc::int32_t = 8;
pub static AMOTION_EVENT_AXIS_VSCROLL: libc::int32_t = 9;
pub static AMOTION_EVENT_AXIS_HSCROLL: libc::int32_t = 10;
pub static AMOTION_EVENT_AXIS_Z: libc::int32_t = 11;
pub static AMOTION_EVENT_AXIS_RX: libc::int32_t = 12;
pub static AMOTION_EVENT_AXIS_RY: libc::int32_t = 13;
pub static AMOTION_EVENT_AXIS_RZ: libc::int32_t = 14;
pub static AMOTION_EVENT_AXIS_HAT_X: libc::int32_t = 15;
pub static AMOTION_EVENT_AXIS_HAT_Y: libc::int32_t = 16;
pub static AMOTION_EVENT_AXIS_LTRIGGER: libc::int32_t = 17;
pub static AMOTION_EVENT_AXIS_RTRIGGER: libc::int32_t = 18;
pub static AMOTION_EVENT_AXIS_THROTTLE: libc::int32_t = 19;
pub static AMOTION_EVENT_AXIS_RUDDER: libc::int32_t = 20;
pub static AMOTION_EVENT_AXIS_WHEEL: libc::int32_t = 21;
pub static AMOTION_EVENT_AXIS_GAS: libc::int32_t = 22;
pub static AMOTION_EVENT_AXIS_BRAKE: libc::int32_t = 23;
pub static AMOTION_EVENT_AXIS_DISTANCE: libc::int32_t = 24;
pub static AMOTION_EVENT_AXIS_TILT: libc::int32_t = 25;
pub static AMOTION_EVENT_AXIS_GENERIC_1: libc::int32_t = 32;
pub static AMOTION_EVENT_AXIS_GENERIC_2: libc::int32_t = 33;
pub static AMOTION_EVENT_AXIS_GENERIC_3: libc::int32_t = 34;
pub static AMOTION_EVENT_AXIS_GENERIC_4: libc::int32_t = 35;
pub static AMOTION_EVENT_AXIS_GENERIC_5: libc::int32_t = 36;
pub static AMOTION_EVENT_AXIS_GENERIC_6: libc::int32_t = 37;
pub static AMOTION_EVENT_AXIS_GENERIC_7: libc::int32_t = 38;
pub static AMOTION_EVENT_AXIS_GENERIC_8: libc::int32_t = 39;
pub static AMOTION_EVENT_AXIS_GENERIC_9: libc::int32_t = 40;
pub static AMOTION_EVENT_AXIS_GENERIC_10: libc::int32_t = 41;
pub static AMOTION_EVENT_AXIS_GENERIC_11: libc::int32_t = 42;
pub static AMOTION_EVENT_AXIS_GENERIC_12: libc::int32_t = 43;
pub static AMOTION_EVENT_AXIS_GENERIC_13: libc::int32_t = 44;
pub static AMOTION_EVENT_AXIS_GENERIC_14: libc::int32_t = 45;
pub static AMOTION_EVENT_AXIS_GENERIC_15: libc::int32_t = 46;
pub static AMOTION_EVENT_AXIS_GENERIC_16: libc::int32_t = 47;

pub static AMOTION_EVENT_BUTTON_PRIMARY: libc::int32_t = 1 << 0;
pub static AMOTION_EVENT_BUTTON_SECONDARY: libc::int32_t = 1 << 1;
pub static AMOTION_EVENT_BUTTON_TERTIARY: libc::int32_t = 1 << 2;
pub static AMOTION_EVENT_BUTTON_BACK: libc::int32_t = 1 << 3;
pub static AMOTION_EVENT_BUTTON_FORWARD: libc::int32_t = 1 << 4;

pub static AMOTION_EVENT_TOOL_TYPE_UNKNOWN: libc::int32_t = 0;
pub static AMOTION_EVENT_TOOL_TYPE_FINGER: libc::int32_t = 1;
pub static AMOTION_EVENT_TOOL_TYPE_STYLUS: libc::int32_t = 2;
pub static AMOTION_EVENT_TOOL_TYPE_MOUSE: libc::int32_t = 3;
pub static AMOTION_EVENT_TOOL_TYPE_ERASER: libc::int32_t = 4;

pub static AINPUT_SOURCE_CLASS_MASK: libc::int32_t = 0x000000ff;

pub static AINPUT_SOURCE_CLASS_NONE: libc::int32_t = 0x00000000;
pub static AINPUT_SOURCE_CLASS_BUTTON: libc::int32_t = 0x00000001;
pub static AINPUT_SOURCE_CLASS_POINTER: libc::int32_t = 0x00000002;
pub static AINPUT_SOURCE_CLASS_NAVIGATION: libc::int32_t = 0x00000004;
pub static AINPUT_SOURCE_CLASS_POSITION: libc::int32_t = 0x00000008;
pub static AINPUT_SOURCE_CLASS_JOYSTICK: libc::int32_t = 0x00000010;

pub static AINPUT_SOURCE_UNKNOWN: libc::int32_t = 0x00000000;

pub static AINPUT_SOURCE_KEYBOARD: libc::int32_t = 0x00000100 | AINPUT_SOURCE_CLASS_BUTTON;
pub static AINPUT_SOURCE_DPAD: libc::int32_t = 0x00000200 | AINPUT_SOURCE_CLASS_BUTTON;
pub static AINPUT_SOURCE_GAMEPAD: libc::int32_t = 0x00000400 | AINPUT_SOURCE_CLASS_BUTTON;
pub static AINPUT_SOURCE_TOUCHSCREEN: libc::int32_t = 0x00001000 | AINPUT_SOURCE_CLASS_POINTER;
pub static AINPUT_SOURCE_MOUSE: libc::int32_t = 0x00002000 | AINPUT_SOURCE_CLASS_POINTER;
pub static AINPUT_SOURCE_STYLUS: libc::int32_t = 0x00004000 | AINPUT_SOURCE_CLASS_POINTER;
pub static AINPUT_SOURCE_TRACKBALL: libc::int32_t = 0x00010000 | AINPUT_SOURCE_CLASS_NAVIGATION;
pub static AINPUT_SOURCE_TOUCHPAD: libc::int32_t = 0x00100000 | AINPUT_SOURCE_CLASS_POSITION;
pub static AINPUT_SOURCE_TOUCH_NAVIGATION: libc::int32_t = 0x00200000 | AINPUT_SOURCE_CLASS_NONE;
pub static AINPUT_SOURCE_JOYSTICK: libc::int32_t = 0x01000000 | AINPUT_SOURCE_CLASS_JOYSTICK;

pub static AINPUT_SOURCE_ANY: libc::int32_t = 0xffffff00;

pub static AINPUT_KEYBOARD_TYPE_NONE: libc::int32_t = 0;
pub static AINPUT_KEYBOARD_TYPE_NON_ALPHABETIC: libc::int32_t = 1;
pub static AINPUT_KEYBOARD_TYPE_ALPHABETIC: libc::int32_t = 2;


/*
 * log.h
 */
#[link(name = "log")]
extern {
    pub fn __android_log_write(prio: libc::c_int, tag: *const libc::c_char,
        text: *const libc::c_char) -> libc::c_int;
}


/*
 * looper.h
 */
pub type ALooper = ();

#[link(name = "android")]
extern {
    pub fn ALooper_forThread() -> *const ALooper;
    pub fn ALooper_acquire(looper: *const ALooper);
    pub fn ALooper_release(looper: *const ALooper);
    pub fn ALooper_prepare(opts: libc::c_int) -> *const ALooper;
    pub fn ALooper_pollOnce(timeoutMillis: libc::c_int, outFd: *mut libc::c_int,
        outEvents: *mut libc::c_int, outData: *mut *mut libc::c_void) -> libc::c_int;
    pub fn ALooper_pollAll(timeoutMillis: libc::c_int, outFd: *mut libc::c_int,
        outEvents: *mut libc::c_int, outData: *mut *mut libc::c_void) -> libc::c_int;
    pub fn ALooper_wake(looper: *const ALooper);
    pub fn ALooper_addFd(looper: *const ALooper, fd: libc::c_int, ident: libc::c_int,
        events: libc::c_int, callback: ALooper_callbackFunc, data: *mut libc::c_void)
        -> libc::c_int;
    pub fn ALooper_removeFd(looper: *const ALooper, fd: libc::c_int) -> libc::c_int;
}

pub static ALOOPER_PREPARE_ALLOW_NON_CALLBACKS: libc::c_int = 1 << 0;

pub static ALOOPER_POLL_WAKE: libc::c_int = -1;
pub static ALOOPER_POLL_CALLBACK: libc::c_int = -2;
pub static ALOOPER_POLL_TIMEOUT: libc::c_int = -3;
pub static ALOOPER_POLL_ERROR: libc::c_int = -4;

pub static ALOOPER_EVENT_INPUT: libc::c_int = 1 << 0;
pub static ALOOPER_EVENT_OUTPUT: libc::c_int = 1 << 1;
pub static ALOOPER_EVENT_ERROR: libc::c_int = 1 << 2;
pub static ALOOPER_EVENT_HANGUP: libc::c_int = 1 << 3;
pub static ALOOPER_EVENT_INVALID: libc::c_int = 1 << 4;

pub type ALooper_callbackFunc = extern fn(libc::c_int, libc::c_int, *mut libc::c_void) -> libc::c_int;


/*
 * native_activity.h
 */
pub type JavaVM = ();
pub type JNIEnv = ();
pub type jobject = *const libc::c_void;

#[repr(C)]
pub struct ANativeActivity {
    pub callbacks: *mut ANativeActivityCallbacks,
    pub vm: *mut JavaVM,
    pub env: *mut JNIEnv,
    pub clazz: jobject,
    pub internalDataPath: *const libc::c_char,
    pub externalDataPath: *const libc::c_char,
    pub sdkVersion: libc::int32_t,
    pub instance: *mut libc::c_void,
    pub assetManager: *mut AAssetManager,
    pub obbPath: *const libc::c_char,
}

#[repr(C)]
pub struct ANativeActivityCallbacks {
    pub onStart: extern fn(*mut ANativeActivity),
    pub onResume: extern fn(*mut ANativeActivity),
    pub onSaveInstanceState: extern fn(*mut ANativeActivity, *mut libc::size_t),
    pub onPause: extern fn(*mut ANativeActivity),
    pub onStop: extern fn(*mut ANativeActivity),
    pub onDestroy: extern fn(*mut ANativeActivity),
    pub onWindowFocusChanged: extern fn(*mut ANativeActivity, libc::c_int),
    pub onNativeWindowCreated: extern fn(*mut ANativeActivity, *const ANativeWindow),
    pub onNativeWindowResized: extern fn(*mut ANativeActivity, *const ANativeWindow),
    pub onNativeWindowRedrawNeeded: extern fn(*mut ANativeActivity, *const ANativeWindow),
    pub onNativeWindowDestroyed: extern fn(*mut ANativeActivity, *const ANativeWindow),
    pub onInputQueueCreated: extern fn(*mut ANativeActivity, *mut AInputQueue),
    pub onInputQueueDestroyed: extern fn(*mut ANativeActivity, *mut AInputQueue),
    pub onContentRectChanged: extern fn(*mut ANativeActivity, *const ARect),
    pub onConfigurationChanged: extern fn(*mut ANativeActivity),
    pub onLowMemory: extern fn(*mut ANativeActivity),
}


/*
 * native_window.h
 */
pub type NativePixmapType = *const libc::c_void;     // FIXME: egl_native_pixmap_t instead
pub type NativeWindowType = *const ANativeWindow;

pub type ANativeWindow = ();


/*
 * rect.h
 */
#[repr(C)]
pub struct ARect {
    pub left: libc::int32_t,
    pub top: libc::int32_t,
    pub right: libc::int32_t,
    pub bottom: libc::int32_t,
}
