#![feature(phase)]

#[phase(plugin, link)]
extern crate android_glue;

android_start!(main)

fn main() {
    android_glue::write_log("main() has been called");
    loop {}
}
