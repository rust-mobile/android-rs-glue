extern crate cargo_apk_injected_glue;

extern {
    fn main(_: isize, _: *const *const u8);
}

// This function is here because we are sure that it will be included by the linker.
// So we call app_dummy in it, in order to be sure that the native glue will be included.
pub fn start(_: isize, _: *const *const u8) -> isize {
    unsafe { cargo_apk_injected_glue::ffi::app_dummy() };
    1
}

#[no_mangle]
#[inline(never)]
#[allow(non_snake_case)]
pub extern "C" fn android_main(app: *mut ()) {
    cargo_apk_injected_glue::android_main2(app as *mut _, move |c, v| unsafe { main(c, v) });
}
