extern crate android_glue;

mod fs;

use std::io::BufRead;

fn main() {
    let f = fs::load("test_asset");
    for line in f.lines() {
        println!("{:?}", line);
    }
    loop {}
}
