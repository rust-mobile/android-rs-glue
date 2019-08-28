cmake_minimum_required(VERSION 2.8.9)
project(cmaketest)

set(CMAKE_BUILD_TYPE Release)
ADD_LIBRARY(cmaketest STATIC cmaketest.c )

install(TARGETS cmaketest DESTINATION .)