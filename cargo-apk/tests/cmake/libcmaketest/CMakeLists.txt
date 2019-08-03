cmake_minimum_required(VERSION 2.8.9)
project(cmaketest)

# TODO: Remove. Specify in build.rs?
set(CMAKE_BUILD_TYPE Release)

# TODO: Cleanup
ADD_LIBRARY(cmaketest STATIC cmaketest.c )

install(TARGETS cmaketest DESTINATION .)