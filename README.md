# Usage

## With docker

The easiest way to compile for Android is to use [docker](https://www.docker.com/) and the
[tomaka/android-rs-glue](https://hub.docker.com/r/tomaka/android-rs-glue/) image.

In order to build an APK, simply do this:

```
docker run --rm -v <path-to-local-directory-with-Cargo.toml>:/root/src tomaka/android-rs-glue cargo apk
```

For example if you're on Linux and you want to compile the project in the current working
directory.

```
docker run --rm -v `pwd`:/root/src -w /root/src tomaka/android-rs-glue cargo apk
```

`cargo apk` only compiles the Cargo.toml of the current working directory, so we have to set it
before running the command. In the future `cargo apk` should be able to accept the
`--manifest-path` option. Do not mount a volume on `/root` or you will erase the local
installation of Cargo.

After the build is finished, you should an Android package in `target/android-artifacts/build/bin`.

## Manual usage

### Setting up your environment

Before you can compile for Android, you need to setup your environment. This needs to be done only once per system.

 - Install [`rustup`](http://rustup.rs).
 - Run `rustup target add arm-linux-androideabi`, or any other target that you want to compile to.

 - Install the Java JDK and Ant (on Ubuntu, `sudo apt-get install openjdk-8-jdk ant`)

 - Download and unzip [the Android NDK](http://developer.android.com/tools/sdk/ndk/index.html)
 - Download and unzip [the Android SDK](http://developer.android.com/sdk/index.html) (under *SDK Tools Only* at the bottom)
 - Update the SDK: `./android-sdk-linux/tools/android update sdk -u`

 - Install `cargo-apk` with `cargo install cargo-apk`.
 - Be sure to set `NDK_HOME` to the path of your NDK and `ANDROID_HOME` to the path of your SDK.

### Compiling

In the project root for your Android crate, run `cargo apk`.

This will build an Android package in `target/android-artifacts/build/bin`.

### Testing on an Android emulator

Start the emulator, then run:

```sh
adb install -r target/your_crate
```

This will install your application on the emulator.

To show log run: `adb logcat | grep RustAndroidGlueStdouterr`

# Interfacing with Android

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
