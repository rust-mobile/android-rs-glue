# Usage

## Setting up your environment

Before you can compile for Android, you need to setup your environment. This needs to be done only once per system.

 - Install [`rustup`](http://rustup.rs).
 - Run `rustup target add arm-linux-androideabi`, or any other target that you want to compile to.

 - Install the Java JDK and Ant (on Ubuntu, `sudo apt-get install openjdk-8-jdk ant`)

 - Download and unzip [the Android NDK](http://developer.android.com/tools/sdk/ndk/index.html)
 - Download and unzip [the Android SDK](http://developer.android.com/sdk/index.html) (under *SDK Tools Only* at the bottom)
 - Update the SDK: `./android-sdk-linux/tools/android update sdk -u`

 - Install `cargo-apk` with `cargo install cargo-apk`.

## Compiling

Run `cargo apk`.

This will build an Android package in `target/android-artifacts/build/bin`.

## Testing on an Android emulator

Start the emulator, then run:

```sh
adb install -r target/your_crate
```

This will install your application on the emulator.

## Interfacing with Android

An application is not very useful if it doesn't have access to the screen, the user inputs, etc.

The `android_glue` crate provides FFI with the Android environment for things that are not in
the stdlib.

# How it works

## The build process

The build process works by invoking `cargo rustc` and:

- Always compiles your crate as a shared library.
- Injects the `android_native_app_glue` file provided by the Android NDK.
- Injects some glue libraries in Rust, which ties the link between `android_native_app_glue` and
  the `main` function of your crate.

This first step outputs a shared library, and is run once per target architecture.

The command then sets up an Android build environment, which includes some Java code, in
`target/android-artifacts` and puts the shared libraries in it. Then it runs `ant`.
