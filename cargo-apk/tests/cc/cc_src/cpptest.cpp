#include <cstdint>
#include <iostream>

extern "C" int32_t multiply_by_four(int32_t value) {
    return value * 4;
}

// Print using std::cout to verify C++ standard library is working properly.
extern "C" void print_value(int32_t value) {
    std::cout << "Value printed from cout: " << value << std::endl;
}