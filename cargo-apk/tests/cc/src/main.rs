fn main() {
    println!("Android cc test");
    let result = unsafe {
        multiply_by_four(add_two(4))
    };

    println!("multiply_by_four(add_two(4)): {}", result);

    println!("Printing value using c++ library:");
    unsafe {
        print_value(result);
    }
}

#[link(name = "ctest")]
extern "C" {
    fn add_two(value : i32) -> i32;
}

#[link(name = "cpptest")]
extern "C" {
    fn multiply_by_four(value : i32) -> i32;
    fn print_value(value : i32) -> std::ffi::c_void;
}

