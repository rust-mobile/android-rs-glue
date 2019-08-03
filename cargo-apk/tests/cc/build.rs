fn main() {
    cc::Build::new()
        .file("./cc_src/ctest.c")
        .compile("ctest");

    cc::Build::new()
        .cpp(true)
        .file("./cc_src/cpptest.cpp")
        .compile("cpptest");
}
