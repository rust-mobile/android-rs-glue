#![feature(libc, set_stdio)]

extern crate libc;

use std::cell::{Cell};
use std::ffi::{CString};
use std::sync::mpsc::{Sender, Receiver, TryRecvError, channel};
use std::sync::Mutex;
use std::thread;
use std::slice;
use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
use std::io::Write;

#[doc(hidden)]
pub mod ffi;

/// This static variable  will store the android_app* on creation, and set it back to 0 at
///  destruction.
/// Apart from this, the static is never written, so there is no risk of race condition.
#[no_mangle]
pub static mut ANDROID_APP: *mut ffi::android_app = 0 as *mut ffi::android_app;

/// This is the structure that serves as user data in the android_app*
#[doc(hidden)]
struct Context {
    senders:    Mutex<Vec<Sender<Event>>>,
    // Any missed events are stored here.
    missed:     Mutex<Vec<Event>>,
    // Better performance to track number of missed items.
    missedcnt:  AtomicUsize,
    // The maximum number of missed events.
    missedmax:  usize,
    // A flag indicating that we should shutdown.
    shutdown:   AtomicBool,
    multitouch: Cell<bool>,
    primary_pointer_id: Cell<i32>,
}

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

#[cfg(not(target_os = "android"))]
use this_platform_is_not_supported;

static mut g_mainthread_boxed: Option<*mut Receiver<()>> = Option::None;

/// Return a tuple with tuple.0 set to true is the application thread
/// has terminated, and tuple.1 set to true if an abnormal exit occured.
fn is_app_thread_terminated() -> (bool, bool) {
    if unsafe { g_mainthread_boxed.is_some() } {
        // Let us see if it had shutdown or paniced.
        let raw = unsafe { g_mainthread_boxed.unwrap() };
        let br: &mut Receiver<()> = unsafe { std::mem::transmute(raw) };
        let result = br.try_recv();
        let terminated = if result.is_err() {
            match result.err().unwrap() {
                TryRecvError::Disconnected => (true, true),
                TryRecvError::Empty => (false, false),
            }
        } else {
            (true, false)
        };
        unsafe { g_mainthread_boxed = Option::Some(raw) };
        terminated
    } else {
        (true, false)
    }
}

/// Return a reference to the application structure.
pub fn get_app<'a>() -> &'a mut ffi::android_app {
    unsafe { std::mem::transmute(ANDROID_APP) }
}

/// This is the function that must be called by `android_main`
#[doc(hidden)]
pub fn android_main2<F>(app: *mut (), main_function: F)
    where F: FnOnce(), F: 'static, F: Send
{
    use std::{mem, ptr};

    write_log("Entering android_main");

    unsafe { ANDROID_APP = std::mem::transmute(app) };
    let app: &mut ffi::android_app = unsafe { std::mem::transmute(app) };

    // creating the context that will be passed to the callback
    let context = Context {
        senders:    Mutex::new(Vec::new()),
        missed:     Mutex::new(Vec::new()),
        missedcnt:  AtomicUsize::new(0),
        missedmax:  1024,
        shutdown:   AtomicBool::new(false),
        multitouch: Cell::new(false),
        primary_pointer_id: Cell::new(0),
    };
    app.onAppCmd = commands_callback;
    app.onInputEvent = inputs_callback;
    app.userData = unsafe { std::mem::transmute(&context) };

    // Set our stdout and stderr so that panics are directed to the log.
    std::io::set_print(Box::new(ToLogWriter::new()));
    std::io::set_panic(Box::new(ToLogWriter::new()));

    // We have to take into consideration that the application we are wrapping
    // may not have been designed for android very well. It may not listen for
    // the destroy command/event, therefore it might not have shutdown and we
    // remained in memory. We need to determine if the thread is still alive.
    let terminated = is_app_thread_terminated();

    if terminated.1 {
        // A little debug message for helping to diagnose problems in your
        // main thread.
        write_log("abnormal exit of main application thread detected");
    }

    // If the thread is still alive we will continue as normal, but we will NOT
    // create the thread. By continuing we keep the application responsive and
    // will not lock up some of the UI, which will be very abnormal for the user,
    // and will result in the entire process being terminated for being unresponsive
    // which is likely the least desired behavior.
    if terminated.0 {
        let (mtx, mrx) = channel::<()>();

        // executing the main function in parallel
        thread::spawn(move || {
            std::io::set_print(Box::new(ToLogWriter::new()));
            std::io::set_panic(Box::new(ToLogWriter::new()));
            main_function();
            mtx.send(()).unwrap();
        });

        // We have to store the JoinGuard off the stack, in the heap, so if we are
        // recalled after a Destroy event/command, then we can make check if the
        // main application thread we created above is still running, and if it is
        // we should wait on it to exit.
        unsafe { g_mainthread_boxed = Option::Some(std::mem::transmute(Box::new(mrx))) };
        write_log("created application thread");
    } else {
        write_log("application thread was still running - not creating new one");
    }

    // Polling for events forever, until shutdown signal is set.
    // note: that this must be done in the same thread as android_main because
    //       ALooper are thread-local
    unsafe {
        loop {
            let mut events = mem::uninitialized();
            let mut source = mem::uninitialized();

            if context.shutdown.load(Ordering::Relaxed) {
                break;
            }

            // A `-1` means to block forever, but any other positive value
            // specifies the number of milliseconds to block for, before
            // returning.
            ffi::ALooper_pollAll(-1, ptr::null_mut(), &mut events,
                &mut source);

            // If the application thread has exited then we need to exit also.
            if is_app_thread_terminated().0 {
                // Not sure exactly how to do this, or what might be the proper
                // manner in which to do it.
                //
                // (1) hide ourselves by switching to home screen
                // (2) display message that we have finished
                // (3) do nothing like we are doing now
                //
                // We must keep this thread going so it can service events, else
                // the user will get a locked UI until the system terminates our
                // process. So we continue processing events..
            }

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
struct ToLogWriter {
    buffer: Vec<u8>,
}

impl ToLogWriter {
    fn new() -> ToLogWriter {
        ToLogWriter {
            buffer: Vec::new(),
        }
    }
}

impl Write for ToLogWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let tag = CString::new("RustAndroidGlueStdouterr").unwrap();
        let tag = tag.as_ptr();
        let mut cursor_id = 0;
        for i in 0 .. buf.len() {
            let c = buf[i];
            if c == '\n' as u8 {
                self.buffer.extend(&buf[cursor_id..i + 1]);
                let message = CString::new(self.buffer.clone()).unwrap();
                let message = message.as_ptr();
                unsafe {
                    ffi::__android_log_write(3, tag, message)
                };
                self.buffer.clear();
                cursor_id = i + 1;
            }
            if i == buf.len() - 1 {
                self.buffer.extend(&buf[cursor_id..i + 1]);
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Send a event to anything that has registered a sender. This is where events
/// messages are sent, and the main application can recieve them from this. There
/// is likely only one sender in our list, but we support more than one.
fn send_event(event: Event) {
    let ctx = get_context();
    let mut senders = ctx.senders.lock().ok().unwrap();

    // Store missed events up to a maximum.
    if senders.len() < 1 {
        // We use a quick target word sized atomic load to check
        if ctx.missedcnt.load(Ordering::SeqCst) < ctx.missedmax {
            let mut missed = ctx.missed.lock().unwrap();
            missed.push(event);
            ctx.missedcnt.fetch_add(1, Ordering::SeqCst);
        }
    }

    senders.retain(|s| s.send(event).is_ok());
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
    let etype = unsafe { ffi::AInputEvent_getType(event) };
    let action = unsafe { ffi::AMotionEvent_getAction(event) };
    let action_code = action & ffi::AMOTION_EVENT_ACTION_MASK;

    match etype {
        ffi::AINPUT_EVENT_TYPE_KEY => match action_code {
            ffi::AKEY_EVENT_ACTION_DOWN => { send_event(Event::EventKeyDown); },
            ffi::AKEY_EVENT_ACTION_UP => send_event(Event::EventKeyUp),
            _ => write_log(&format!("unknown input-event-type:{} action_code:{}", etype, action_code)),
        },
        ffi::AINPUT_EVENT_TYPE_MOTION => {
            let motion_action = match action_code {
                ffi::AMOTION_EVENT_ACTION_DOWN |
                ffi::AMOTION_EVENT_ACTION_POINTER_DOWN => MotionAction::Down,
                ffi::AMOTION_EVENT_ACTION_UP |
                ffi::AMOTION_EVENT_ACTION_POINTER_UP => MotionAction::Up,
                ffi::AMOTION_EVENT_ACTION_MOVE => MotionAction::Move,
                ffi::AMOTION_EVENT_ACTION_CANCEL => MotionAction::Cancel,
                _ => {
                    write_log(&format!("unknown action_code:{}", action_code));
                    return 0
                }
            };
            let context = get_context();
            let idx = ((action & ffi::AMOTION_EVENT_ACTION_POINTER_INDEX_MASK)
                       >> ffi::AMOTION_EVENT_ACTION_POINTER_INDEX_SHIFT)
                      as libc::size_t;

            let pointer_id = unsafe { ffi::AMotionEvent_getPointerId(event, idx) };
            if action_code == ffi::AMOTION_EVENT_ACTION_DOWN {
                context.primary_pointer_id.set(pointer_id);
            }
            let primary_pointer_id = context.primary_pointer_id.get();
            let multitouch = context.multitouch.get();

            match motion_action {
                MotionAction::Down | MotionAction::Up | MotionAction::Cancel => {
                    if multitouch || pointer_id == primary_pointer_id {
                        send_event(Event::EventMotion(Motion {
                            action: motion_action,
                            pointer_id: pointer_id,
                            x: unsafe { ffi::AMotionEvent_getX(event, idx) },
                            y: unsafe { ffi::AMotionEvent_getY(event, idx) },
                        }));
                    }
                }
                MotionAction::Move => {
                    // A move event may have multiple changed pointers. Send an event for each.
                    let pointer_count = unsafe { ffi::AMotionEvent_getPointerCount(event) };
                    for idx in 0..pointer_count {
                        let pointer_id = unsafe { ffi::AMotionEvent_getPointerId(event, idx) };
                        if multitouch || pointer_id == primary_pointer_id {
                            send_event(Event::EventMotion(Motion {
                                action: motion_action,
                                pointer_id: pointer_id,
                                x: unsafe { ffi::AMotionEvent_getX(event, idx) },
                                y: unsafe { ffi::AMotionEvent_getY(event, idx) },
                            }));
                        }
                    }
                }
            }
        },
        _ => write_log(&format!("unknown input-event-type:{} action_code:{}", etype, action_code)),
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
        ffi::APP_CMD_DESTROY => {
            send_event(Event::Destroy);
            context.shutdown.store(true, Ordering::Relaxed);
        },
        _ => write_log(&format!("unknown command {}", command)),
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

pub fn set_multitouch(multitouch: bool) {
    get_context().multitouch.set(multitouch);
}

/// Adds a sender where events will be sent to, but also sends
/// any missing events to the sender object.
///
/// The missing events happen when the application starts, but before
/// any senders are registered. Since these might be important to certain
/// applications, this function provides that support.
pub fn add_sender_missing(sender: Sender<Event>) {
    let ctx = get_context();
    let mut senders = ctx.senders.lock().ok().unwrap();

    if senders.len() == 0 {
        // If the first sender added then, let us send any missing events.
        let mut missed = ctx.missed.lock().unwrap();
        while missed.len() > 0 {
            sender.send(missed.remove(0)).unwrap();
        }
        ctx.missedcnt.store(0, Ordering::Relaxed);
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
        thread::sleep_ms(10);
    }
}

///
pub fn write_log(message: &str) {
    let message = CString::new(message).unwrap();
    let message = message.as_ptr();
    let tag = CString::new("RustAndroidGlueStdouterr").unwrap();
    let tag = tag.as_ptr();
    unsafe { ffi::__android_log_write(3, tag, message) };
}

pub enum AssetError {
    AssetMissing,
    EmptyBuffer,
}

pub fn load_asset(filename: &str) -> Result<Vec<u8>, AssetError> {
    struct AssetCloser {
        asset: *mut ffi::Asset,
    }

    impl Drop for AssetCloser {
        fn drop(&mut self) {
            unsafe {
                ffi::AAsset_close(self.asset)
            };
        }
    }

    unsafe fn get_asset_manager() -> *mut ffi::AAssetManager {
        let app = &*ANDROID_APP;
        let activity = &*app.activity;
        activity.assetManager
    }

    let filename_c_str = CString::new(filename).unwrap();
    let filename_c_str = filename_c_str.as_ptr();
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
        slice::from_raw_parts(buff as *const u8, len as usize).to_vec()
    };
    Ok(vec)
}
