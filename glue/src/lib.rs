#![feature(macro_rules)]
#![feature(phase)]

#![unstable]

#[phase(plugin)]
extern crate compile_msg;

extern crate libc;

#[doc(hidden)]
pub mod ffi;

/// This static variable is public because it needs to be accessible from the
///  macros in user code.
///
/// It will store the android_app* on creation, and set it back to 0 at destruction.
/// Apart from this, the static is never written, so there is no risk of race condition.
#[doc(hidden)]
pub static mut android_app: *mut ffi::android_app = 0 as *mut ffi::android_app;

#[cfg(not(target_os = "android"))]
compile_note!("You are not compiling for Android")

#[macro_export]
macro_rules! android_start(
    ($main: ident) => (
        pub mod __android_start {
            extern crate android_glue;
            extern crate libc;
            extern crate native;

            // this function is here because we are sure that it will be included by the linker
            // so we call app_dummy in it, in order to be sure that the native glue will be included
            #[start]
            pub fn start(_: int, _: *const *const u8) -> int {
                unsafe { android_glue::ffi::app_dummy() };
                1
            }

            #[no_mangle]
            #[inline(never)]
            #[allow(non_snake_case)]
            pub extern "C" fn android_main(app: *mut android_glue::ffi::android_app) {
                use self::native::NativeTaskBuilder;
                use std::task::TaskBuilder;

                unsafe { android_glue::android_app = app };

                android_glue::write_log("ANativeActivity_onCreate has been called");

                unsafe { android_glue::ffi::app_dummy() };

                native::start(1, &b"".as_ptr(), proc() {
                    super::$main();
                });

                unsafe { android_glue::android_app = 0 as *mut android_glue::ffi::android_app };
            }
        }
    )
)

/// Returns a handle to the native window.
pub unsafe fn get_native_window() -> ffi::NativeWindowType {
    if android_app.is_null() {
        fail!("The application was not initialized from android_main");
    }

    (*android_app).window
}

/// 
pub fn write_log(message: &str) {
    message.with_c_str(|message| {
        b"RustAndroidGlue".with_c_str(|tag| {
            unsafe { ffi::__android_log_write(3, tag, message) };
        });
    });
}
