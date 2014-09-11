# What is this repository?

This repository consists in two crates: a binary named `apk-builder`, and a library named `android_glue`.

`apk-builder` is a wrapper around `gcc` and the Android SDK and NDK. Invoking it will produce an `.apk` (Android package) instead of a regular binary.

This linker is supposed to be used alongside with `android_glue`, which is a low-level library that will allow you to access the Android environment (the window, the events, etc.). `android_glue` is supposed to be used as a dependency for higher-level libraries that require access to this environment, like [`gl-init`](https://github.com/tomaka/gl-init-rs).

# Installation

You can make *any* program run on Android with the following steps.

First, add a dependency to `android_glue`:

```toml
[dependencies.android_glue]
git = "https://github.com/tomaka/android-rs-glue"
```

Then, add `extern crate android_glue` and invoke `android_start!` in your main crate.

```rust
#[cfg(target_os = "android")]
#[phase(plugin, link)] 
extern crate android_glue;

#[cfg(target_os = "android")]
android_start!(main)

fn main() {
    // ...
}
```

Then, clone or download this repository somewhere on your computer and compile `apk-builder`.

```sh
git clone https://github.com/tomaka/android-rs-glue apk-builder
cd apk-builder/apk-builder
cargo build
```

Finally, add a file named [`.cargo/config`](http://crates.io/config.html) in your main repository in order to ask rustc to use `apk-builder`:

```toml
[target.arm-linux-androideabi]
linker = "apk-builder/apk-builder/target/apk-builder"
```

Instead of a regular binary, compiling with `cargo build --target=arm-linux-androideabi` will produce an APK that can be installed on an Android device. See `How to compile` below.

One important thing to notice is that this doesn't break your existing build. Calling `cargo build` or `cargo build --target=something-something-something` will produce the exact same thing as before.

# Usage

 - `android_start!(main)` defines the entry point of your application to `main`.

The library provides other unsafe low-level functions, but they should only be used by higher-level libraries that need access to the Android environment.

# How to compile

## Setting up your environment

 - If you are on Linux 64 bits, install the 32 bits binaries (`apt-get install libc6-i386 lib32z1 lib32stdc++6`)

 - Download and unzip [the Android NDK](http://developer.android.com/tools/sdk/ndk/index.html)
 - Generate a stand-alone toolchain of the NDK, example: `./android-ndk-r10/build/tools/make-standalone-toolchain.sh --platform=android-L --toolchain=arm-linux-androideabi --install-dir=/opt/ndk_standalone --ndk-dir=/home/you/android-ndk-r10`

 - Clone the Rust compiler: `git clone https://github.com/rust-lang/rust.git`
 - Compile Rust for Android: `mkdir rust-build`, `cd rust-build`, `../rust/configure --target=arm-linux-androideabi --android-cross-path=/opt/ndk_standalone`, `make`, `make install`

 - Download and unzip [Cargo](https://github.com/rust-lang/cargo#installing-cargo-from-nightlies)
 - Install Cargo: `./cargo-nightly-x86_64-unknown-linux-gnu/install.sh`

 - Install the Java JDK and Ant (`apt-get install openjdk-7-jdk ant`)

 - Download and unzip [the Android SDK](http://developer.android.com/sdk/index.html) (under *SDK tools* in *VIEW ALL DOWNLOADS AND SIZES*)
 - Update the SDK: `./android-sdk-linux/tools/android update sdk -u`

## Building your project

Building your project is done in one single step:

`ANDROID_HOME=/path/to/android/sdk NDK_HOME=/path/to/ndk NDK_STANDALONE=/opt/ndk_standalone cargo build --target=arm-linux-androideabi`

(Note: the SDK installer should automatically set `ANDROID_HOME`, in which case you don't need to pass it. The NDK standalone may be by-passed in the future, making `NDK_HOME` the only environment variable that will need to be passed).

This will generate a file named `target/arm-linux-androideabi/your_crate` or `target/arm-linux-androideabi/your_crate.exe`. Even though it has the wrong extension, this file is an Android package (`.apk`) that can be installed on an Android device.

# Testing on an Android emulator

Start the emulator, then run:

```sh
adb install -r target/your_crate
```

This will install your application on the emulator.

For the moment, your application's name is always the name of your crate. This will be customizable in the future.
