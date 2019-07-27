fn main() {
    #[cfg(target_os = "android")]
    android_glue::write_log("main() has been called on the secondary binary");
    loop {}
}
