#!/bin/bash

set -e

fail() {
    echo "$@"
    exit 1
}

[[ $(basename $(pwd)) = cargo-apk ]] || fail "Must be run in cargo-apk directory"

do_test() {
    do_package "tests/$1" || fail "Compiling test $1 failed"
    check_symbols "tests/$1" || fail "onCreate not found in test $1"
}

do_example() {
    do_package "../examples/$1" || fail "Compiling example $1 failed"
    check_symbols "../examples/$1" || fail "onCreate not found in example $1"
}

do_package() {
    pushd "$1" >/dev/null
    cargo apk build --all-targets || return 1
    cargo apk build --all-targets --release || return 1
    popd >/dev/null
}

check_symbols() {
    (find "$1/target/android-artifacts/release/bin" -name *.so && \
     find "$1/target/android-artifacts/debug/bin" -name *.so) | \
        while read f ; do
            nm -Dg --defined-only "$f" | cut -f3 -d' ' | grep -qxF ANativeActivity_onCreate || return 1
        done
}

do_example advanced
do_example basic
do_example multiple_targets
do_example use_assets
do_example use_icon
do_test inner_attributes
do_test native-library
do_test cc
do_test cmake
