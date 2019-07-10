#![no_std]

extern {
    fn main(_: isize, _: *const *const u8);
}

static mut ANDROID_APP: *mut () = 0 as *mut ();

#[no_mangle]
pub extern "C" fn android_main(app: *mut ()) {
    unsafe {
        ANDROID_APP = app;
        let argc = 1;
        let argv = &(b"android\0" as *const u8);
        main(argc, argv);
    }
}

pub extern "C" fn get_android_app() -> *mut () {
    unsafe {
        ANDROID_APP
    }
}
