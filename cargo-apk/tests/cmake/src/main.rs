fn main() {
    println!("Android cmake test");
    let result = unsafe {
        multiply_by_10(2)
    };

    println!("multiply_by_10(2): {}", result);
}


#[link(name = "cmaketest")]
extern "C" {
    fn  multiply_by_10(value : i32) -> i32;
}
