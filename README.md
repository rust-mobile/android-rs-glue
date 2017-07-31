[![Become a patron](https://c5.patreon.com/external/logo/become_a_patron_button.png)](https://www.patreon.com/tomaka)

# Usage

## With docker

The easiest way to compile for Android is to use [docker](https://www.docker.com/) and the
[tomaka/cargo-apk](https://hub.docker.com/r/tomaka/cargo-apk/) image.

In order to build an APK, simply do this:

```
docker run --rm -v <path-to-local-directory-with-Cargo.toml>:/root/src tomaka/cargo-apk cargo apk build
```

For example if you're on Linux and you want to compile the project in the current working
directory.

```
docker run --rm -v "$(pwd):/root/src" -w /root/src tomaka/cargo-apk cargo apk build
```

Do not mount a volume on `/root` or you will erase the local installation of Cargo.

After the build is finished, you should get an Android package in `target/android-artifacts/app/build/outputs/apk`.

## Manual usage

### Setting up your environment

Before you can compile for Android, you need to setup your environment. This needs to be done only once per system.

 - Install [`rustup`](http://rustup.rs).
 - Run `rustup target add arm-linux-androideabi`, or any other target that you want to compile to.

 - Install the Java JDK (on Ubuntu, `sudo apt-get install openjdk-8-jdk`)
 - [Install Gradle](https://gradle.org/install/).

 - Download and unzip [the Android NDK](http://developer.android.com/tools/sdk/ndk/index.html)
 - Download and unzip [the Android SDK](http://developer.android.com/sdk/index.html) (under *SDK Tools Only* at the bottom)
 - Install some components in the SDK: `./android-sdk/tools/bin/sdkmanager "platform-tools" "platforms;android-18" "build-tools;26.0.1"`

 - Install `cargo-apk` with `cargo install cargo-apk`.
 - Set the environment variables `NDK_HOME` to the path of the NDK and `ANDROID_HOME` to the path of the SDK.

### Compiling

In the project root for your Android crate, run `cargo apk build`. You can use the same options as
with the regular `cargo build`.

This will build an Android package in `target/android-artifacts/app/build/outputs/apk`.

### Testing on an Android emulator

Start the emulator, then run:

```sh
cargo apk run
```

This will install your application on the emulator, then run it.  
If you only want to install, use `cargo apk install`.

To show log run: `cargo apk logcat | grep RustAndroidGlueStdouterr`

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
`target/android-artifacts` and puts the shared libraries in it. Then it runs `gradle`.

# Supported `[package.metadata.android]` entries

```toml
[package.metadata.android]

# The Java package name for your application.
# Hyphens are converted to underscores.
package_name = "com.author-name.my-android-app"

# The user-friendly name for your app, as displayed in the applications menu.
label = "My Android App"

# Path to your application's res/ folder. See `examples/use_icon/res`.
res = "path/to/res_folder"

# Virtual path your application's icon for any mipmap level. See `examples/use_icon/icon`.
icon = "@mipmap/ic_laucher"

# Path to the folder containing your application's assets. See `examples/use_assets/assets`.
assets = "path/to/assets_folder"

# The target Android API level.
# It defaults to 18 because this is the minimum supported by rustc.
android_version = 18

# If set to true, makes the app run in full-screen, by adding the following line
# as an XML attribute to the manifest's <application> tag :
#     android:theme="@android:style/Theme.DeviceDefault.NoActionBar.Fullscreen
# Defaults to false.
fullscreen = false

# Specifies the array of targets to build for.
# Defaults to "arm-linux-androideabi".
# Other possible targets include "aarch64-linux-android", 
# "armv7-linux-androideabi", "i686-linux-android" and "x86_64-linux-android".
build_targets = [ "arm-linux-androideabi", "armv7-linux-androideabi" ]

# The maximum supported OpenGL ES version , as claimed by the manifest. Defaults to 2.0.
# See https://developer.android.com/guide/topics/graphics/opengl.html#manifest
opengles_version_major = 2
opengles_version_minor = 0

# Adds extra arbitrary XML attributes to the <application> tag in the manifest.
# See https://developer.android.com/guide/topics/manifest/application-element.html
[package.metadata.android.application_attributes]
"android:debuggable" = "true"
"android:hardwareAccelerated" = "true"

# Adds extra arbitrary XML attributes to the <activity> tag in the manifest.
# See https://developer.android.com/guide/topics/manifest/activity-element.html
[package.metadata.android.activity_attributes]
"android:screenOrientation" = "unspecified"
"android:uiOptions" = "none"
```
