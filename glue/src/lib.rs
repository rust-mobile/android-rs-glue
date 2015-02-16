//!
//! You can make _any_ program run on Android with minimal non-intrusive 
//! modification using this library.
//!
//! If you are using Cargo add this line to your `Cargo.toml`.
//!
//!     [dependencies.android_glue] git = "https://github.com/tomaka/android-rs-glue"
//!
//! If you are not using Cargo then make sure the library is findable by `rustc`
//! and then see example program below.
//!
//!
//! A sample application:
//!
//!     // This code will only be included if android is the target.
//!     #[cfg(target_os = "android")]
//!     #[macro_use]
//!     extern crate android_glue;
//!     // This code will only be included if android is the target.
//!     #[cfg(target_os = "android")]
//!     android_start!(main);
//!     
//!     use std::sync::mpsc::channel;
//!     use android_glue::{Event, add_sender};
//!     
//!     fn main() {
//!         // Create a channel.
//!         let (eventstx, eventsrx) = channel::<Event>();
//!         
//!         // Try `dbg logcat *:D | grep RustAndroidGlue` when you run this program.    
//!         println!("HELLO WORLD");
//!         
//!         // Add the sender half of the channel so we can be sent events.
//!         add_sender(eventstx);
//!         
//!         loop {
//!             // Print the event since it implements the Debug trait.
//!             println!("{:?}", eventsrx.recv());
//!         }
//!     }

#![feature(box_syntax, plugin, libc, core, io, collections, std_misc)]

#![unstable]

extern crate libc;

use std::ffi::{CString};
use std::sync::mpsc::{Sender};
use std::sync::Mutex;
use std::thread::Thread;

#[doc(hidden)]
pub mod ffi;

/// This static variable  will store the android_app* on creation, and set it back to 0 at
///  destruction.
/// Apart from this, the static is never written, so there is no risk of race condition.
static mut ANDROID_APP: *mut ffi::android_app = 0 as *mut ffi::android_app;

/// This is the structure that serves as user data in the android_app*
#[doc(hidden)]
struct Context {
    senders:    Mutex<Vec<Sender<Event>>>,
    // Any missed events are stored here.
    missed:     Mutex<Vec<Event>>,
    // The maximum number of missed events.
    missedmax:  usize,
}

/// An event triggered by the Android environment.
#[derive(Debug)]
pub enum Event {
    EventUp,
    EventDown,
    EventMove(i32, i32),
    // The above are more specifically EventMotion, but to prevent a breaking
    // change I did not rename them, but instead made EventKey** --kmcg3413@gmail.com
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

impl Copy for Event {}

#[cfg(not(target_os = "android"))]
use this_platform_is_not_supported;

#[macro_export]
macro_rules! android_start(
    ($main: ident) => (
        pub mod __android_start {
            extern crate android_glue;

            // this function is here because we are sure that it will be included by the linker
            // so we call app_dummy in it, in order to be sure that the native glue will be included
            #[start]
            pub fn start(_: isize, _: *const *const u8) -> isize {
                unsafe { android_glue::ffi::app_dummy() };
                1
            }


            #[no_mangle]
            #[inline(never)]
            #[allow(non_snake_case)]
            pub extern "C" fn android_main(app: *mut ()) {
                android_glue::android_main2(app, move|| super::$main());
            }
        }
    )
);

/// This is the function that must be called by `android_main`
#[doc(hidden)]
pub fn android_main2<F>(app: *mut (), main_function: F)
    where F: FnOnce(), F: Send
{
    use std::{mem, ptr};

    write_log("Entering android_main");

    unsafe { ANDROID_APP = std::mem::transmute(app) };
    let app: &mut ffi::android_app = unsafe { std::mem::transmute(app) };

    // creating the context that will be passed to the callback
    let context = Context { 
        senders:    Mutex::new(Vec::new()),
        missed:     Mutex::new(Vec::new()),
        missedmax:  1024,           
    };
    app.onAppCmd = commands_callback;
    app.onInputEvent = inputs_callback;
    app.userData = unsafe { std::mem::transmute(&context) };

    // Set our stdout and stderr so that panics are directed to the log.
    std::old_io::stdio::set_stdout(box std::old_io::LineBufferedWriter::new(ToLogWriter));
    std::old_io::stdio::set_stderr(box std::old_io::LineBufferedWriter::new(ToLogWriter));

    // executing the main function in parallel
    let g = Thread::spawn(move|| {
        std::old_io::stdio::set_stdout(box std::old_io::LineBufferedWriter::new(ToLogWriter));
        std::old_io::stdio::set_stderr(box std::old_io::LineBufferedWriter::new(ToLogWriter));
        main_function()
    });

    // polling for events forever
    // note that this must be done in the same thread as android_main because ALooper are
    //  thread-local
    unsafe {
        loop {
            let mut events = mem::uninitialized();
            let mut source = mem::uninitialized();

            // passing -1 means that we are blocking
            let ident = ffi::ALooper_pollAll(-1, ptr::null_mut(), &mut events,
                &mut source);

            // processing the event
            if !source.is_null() {
                let source: *mut ffi::android_poll_source = mem::transmute(source);
                ((*source).process)(ANDROID_APP, source);
            }
        }
    }

    // terminating the application
    unsafe { ANDROID_APP = 0 as *mut ffi::android_app };
}

/// Writer that will redirect what is written to it to the logs.
struct ToLogWriter;

impl Writer for ToLogWriter {
    fn write_all(&mut self, buf: &[u8]) -> std::old_io::IoResult<()> {
        let message = CString::from_slice(buf);
        let message = message.as_slice_with_nul().as_ptr();
        let tag = b"RustAndroidGlueStdouterr";
        let tag = CString::from_slice(tag);
        let tag = tag.as_slice_with_nul().as_ptr();
        unsafe { ffi::__android_log_write(3, tag, message) };
        Ok(())
    }
}

/// Send a event to anything that has registered a sender. This is where events
/// messages are sent, and the main application can recieve them from this. There
/// is likely only one sender in our list, but we support more than one.
fn send_event(event: Event) {
    let mut ctx = get_context();
    let senders = ctx.senders.lock().ok().unwrap();

    // Store missed events up to a maximum. This is a little expensive, because
    // we have to double lock, until a sender is added. For applications that 
    // never register a sender it would always double lock on any event, but 
    // those applications might be short lived and not bothered? 
    // -- kmcg3413@gmail.com
    if senders.len() < 1 {
        let mut missed = ctx.missed.lock().unwrap();
        if missed.len() < ctx.missedmax {
            missed.push(event);
        }
    }

    for sender in senders.iter() {
        sender.send(event);
    }
}

/// The callback for input.
///
/// This callback is registered when we startup and is called by our main thread,
/// from the function `android_main2`. We then process the event to gain additional
/// information, and finally send the event, which normally would be recieved by
/// the main application thread IF it has registered a sender. 
pub extern fn inputs_callback(_: *mut ffi::android_app, event: *const ffi::AInputEvent)
    -> libc::int32_t
{
    fn get_xy(event: *const ffi::AInputEvent) -> (i32, i32) {
        let x = unsafe { ffi::AMotionEvent_getX(event, 0) };
        let y = unsafe { ffi::AMotionEvent_getY(event, 0) };
        (x as i32, y as i32)
    }

    let etype = unsafe { ffi::AInputEvent_getType(event) };
    let action = unsafe { ffi::AMotionEvent_getAction(event) };
    let action_code = action & ffi::AMOTION_EVENT_ACTION_MASK;

    match etype {
        ffi::AINPUT_EVENT_TYPE_KEY => match action_code {
            ffi::AKEY_EVENT_ACTION_DOWN => { send_event(Event::EventKeyDown); },
            ffi::AKEY_EVENT_ACTION_UP => send_event(Event::EventKeyUp),
            _ => write_log(format!("unknown input-event-type:{} action_code:{}", etype, action_code).as_slice()),
        },
        ffi::AINPUT_EVENT_TYPE_MOTION => match action_code {
            ffi::AMOTION_EVENT_ACTION_UP
                | ffi::AMOTION_EVENT_ACTION_OUTSIDE
                | ffi::AMOTION_EVENT_ACTION_CANCEL
                | ffi::AMOTION_EVENT_ACTION_POINTER_UP =>
            {
                send_event(Event::EventUp);
            },
            ffi::AMOTION_EVENT_ACTION_DOWN
                | ffi::AMOTION_EVENT_ACTION_POINTER_DOWN =>
            {
                let (x, y) = get_xy(event);
                send_event(Event::EventMove(x, y));
                send_event(Event::EventDown);
            },
            _ => {
                let (x, y) = get_xy(event);
                send_event(Event::EventMove(x, y));
            },
        },
        _ => write_log(format!("unknown input-event-type:{} action_code:{}", etype, action_code).as_slice()),
    }
    0
}

/// The callback for commands.
#[doc(hidden)]
pub extern fn commands_callback(_: *mut ffi::android_app, command: libc::int32_t) {
    let context = get_context();

    match command {
        ffi::APP_CMD_INIT_WINDOW => send_event(Event::InitWindow),
        ffi::APP_CMD_SAVE_STATE => send_event(Event::SaveState),
        ffi::APP_CMD_TERM_WINDOW => send_event(Event::TermWindow),
        ffi::APP_CMD_GAINED_FOCUS => send_event(Event::GainedFocus),
        ffi::APP_CMD_LOST_FOCUS => send_event(Event::LostFocus),
        ffi::APP_CMD_INPUT_CHANGED => send_event(Event::InputChanged),
        ffi::APP_CMD_WINDOW_RESIZED => send_event(Event::WindowResized),
        ffi::APP_CMD_WINDOW_REDRAW_NEEDED => send_event(Event::WindowRedrawNeeded),
        ffi::APP_CMD_CONTENT_RECT_CHANGED => send_event(Event::ContentRectChanged),
        ffi::APP_CMD_CONFIG_CHANGED => send_event(Event::ConfigChanged),
        ffi::APP_CMD_LOW_MEMORY => send_event(Event::LowMemory),
        ffi::APP_CMD_START => send_event(Event::Start),
        ffi::APP_CMD_RESUME => send_event(Event::Resume),
        ffi::APP_CMD_PAUSE => send_event(Event::Pause),
        ffi::APP_CMD_STOP => send_event(Event::Stop),
        ffi::APP_CMD_DESTROY => send_event(Event::Destroy),
        _ => write_log(format!("unknown command {}", command).as_slice()),
    }
}

/// Returns the current Context.
fn get_context() -> &'static Context {
    let context = unsafe { (*ANDROID_APP).userData };
    unsafe { std::mem::transmute(context) }
}

/// Adds a sender where events will be sent to.
pub fn add_sender(sender: Sender<Event>) {
    get_context().senders.lock().unwrap().push(sender);
}

/// Adds a sender where events will be sent to, but also sends
/// any missing events to the sender object. 
///
/// The missing events happen when the application starts, but before
/// any senders are registered. Since these might be important to certain
/// applications, this function provides that support.
pub fn add_sender_missing(sender: Sender<Event>) {
    let mut ctx = get_context();
    let mut senders = ctx.senders.lock().ok().unwrap();

    if senders.len() == 0 {
        // If the first sender added then, let us send any missing events.
        let mut missed = ctx.missed.lock().unwrap();
        while missed.len() > 0 {
            sender.send(missed.remove(0));
        }
    }

    senders.push(sender);
}

/// Returns a handle to the native window.
pub unsafe fn get_native_window() -> ffi::NativeWindowType {
    if ANDROID_APP.is_null() {
        panic!("The application was not initialized from android_main");
    }

    loop {
        let value = (*ANDROID_APP).window;
        if !value.is_null() {
            return value;
        }

        // spin-locking
        std::old_io::timer::sleep(std::time::Duration::milliseconds(10));
    }
}

/// 
pub fn write_log(message: &str) {
    let message = CString::from_slice(message.as_bytes());
    let message = message.as_slice_with_nul().as_ptr();
    let tag = b"RustAndroidGlueStdouterr";
    let tag = CString::from_slice(tag);
    let tag = tag.as_slice_with_nul().as_ptr();
    unsafe { ffi::__android_log_write(3, tag, message) };
}

pub enum AssetError {
    AssetMissing,
    EmptyBuffer,
}

pub fn load_asset(filename: &str) -> Result<Vec<u8>, AssetError> {
    struct AssetCloser {
        asset: *const ffi::Asset,
    }

    impl Drop for AssetCloser {
        fn drop(&mut self) {
            unsafe {
                ffi::AAsset_close(self.asset)
            };
        }
    }

    unsafe fn get_asset_manager() -> *const ffi::AAssetManager {
        let app = &*ANDROID_APP;
        let activity = &*app.activity;
        activity.assetManager
    }

    let filename_c_str = CString::from_slice(filename.as_bytes());
    let filename_c_str = filename_c_str.as_slice_with_nul().as_ptr();
    let asset = unsafe {
        ffi::AAssetManager_open(
            get_asset_manager(), filename_c_str, ffi::MODE_STREAMING)
    };
    if asset.is_null() {
        return Err(AssetError::AssetMissing);
    }
    let _asset_closer = AssetCloser{asset: asset};
    let len = unsafe {
        ffi::AAsset_getLength(asset)
    };
    let buff = unsafe {
        ffi::AAsset_getBuffer(asset)
    };
    if buff.is_null() {
        return Err(AssetError::EmptyBuffer);
    }
    let vec = unsafe {
        Vec::from_raw_buf(buff as *const u8, len as usize)
    };
    Ok(vec)
}
