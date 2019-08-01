[![Become a patron](https://c5.patreon.com/external/logo/become_a_patron_button.png)](https://www.patreon.com/tomaka)

# Usage

## With Docker

The easiest way to compile for Android is to use [Docker](https://www.docker.com/) and the
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

After the build is finished, you should get an Android package in `target/android-artifacts/debug/apk`.

## Manual usage

### Setting up your environment

Before you can compile for Android, you need to setup your environment. This needs to be done only once per system.

 - Install [`rustup`](https://rustup.rs/).
 - Run `rustup target add <target>` for all supported targets to which you want to compile. Building will attempt to build for all supported targets unless the build targets are adjusted via `Cargo.toml`.
    - `rustup target add armv7-linux-androideabi`
    - `rustup target add aarch64-linux-android`
    - `rustup target add i686-linux-android`
    - `rustup target add x86_64-linux-android`
 - Install the Java JRE or JDK (on Ubuntu, `sudo apt-get install openjdk-8-jdk`).
 - Download and unzip [the Android NDK](https://developer.android.com/ndk).
 - Download and unzip [the Android SDK](https://developer.android.com/studio).
 - Install some components in the SDK: `./android-sdk/tools/bin/sdkmanager "platform-tools" "platforms;android-29" "build-tools;29.0.0"`.
 - Install `cargo-apk` with `cargo install cargo-apk`.
 - Set the environment variables `NDK_HOME` to the path of the NDK and `ANDROID_HOME` to the path of the SDK.

### Compiling

In the project root for your Android crate, run `cargo apk build`. You can use the same options as
with the regular `cargo build`.

This will build an Android package in `target/android-artifacts/<debug|release>/apk`.

### Compiling Multiple Binaries

`cargo apk build` supports building multiple binaries and examples using the same arguments as `cargo build`. It will produce an APK for each binary.

Android packages for bin targets are placed in `target/android-artifacts/<debug|release>/apk`.

Android packages for example targets are placed in `target/android-artifacts/<debug|release>/apk/examples`.

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

The build process works by running rustc and:

- Always compiles your crate as a static library.
- Uses `ndk-build` provided by the NDK to to build a shared library.
- Links to the `android_native_app_glue` library provided by the Android NDK.
- Injects some glue libraries in Rust, which ties the link between `android_native_app_glue` and the `main` function of your crate.

This first step outputs a shared library, and is run once per target architecture.

The command then builds the APK using the shared library, generated manifest, and tools from the Android SDK. 
It signs the APK with the default debug keystore used by Android development tools. If the keystore doesn't exist, it creates it using the keytool from the JRE or JDK.

# Supported `[package.metadata.android]` entries

```toml
# The target Android API level.
# "android_version" is the compile SDK version. It defaults to 29.
# (target_sdk_version defaults to the value of "android_version")
# (min_sdk_version defaults to 18) It defaults to 18 because this is the minimum supported by rustc.
android_version = 29
target_sdk_version = 29
min_sdk_version = 26

# Specifies the array of targets to build for.
# Defaults to "armv7-linux-androideabi", "aarch64-linux-android", "i686-linux-android".
build_targets = [ "armv7-linux-androideabi", "aarch64-linux-android", "i686-linux-android", "x86_64-linux-android" ]

#
# The following value can be customized on a per bin/example basis. See multiple_targets example
# If a value is not specified for a secondary target, it will inherit the value defined in the `package.metadata.android`
# section unless otherwise noted.
#

# The Java package name for your application.
# Hyphens are converted to underscores.
# Defaults to rust.<target_name> for binaries. 
# Defaults to rust.<package_name>.example.<target_name> for examples.
# For example: for a binary "my_app", the default package name will be "rust.my_app"
# Secondary targets will not inherit the value defined in the root android configuration.
package_name = "rust.cargo.apk.advanced"

# The user-friendly name for your app, as displayed in the applications menu.
# Defaults to the target name
# Secondary targets will not inherit the value defined in the root android configuration.
label = "My Android App"

# Internal version number used to determine whether one version is more recent than another. Must be an integer.
# Defaults to 1
# See https://developer.android.com/guide/topics/manifest/manifest-element
version_code = 2

# The version number shown to users.
# Defaults to the cargo package version number
# See https://developer.android.com/guide/topics/manifest/manifest-element
version_name = "2.0"

# Path to your application's resources folder.
# If not specified, resources will not be included in the APK
res = "path/to/res_folder"

# Virtual path your application's icon for any mipmap level.
# If not specified, an icon will not be included in the APK.
icon = "@mipmap/ic_launcher"

# Path to the folder containing your application's assets.
# If not specified, assets will not be included in the APK
assets = "path/to/assets_folder"

# If set to true, makes the app run in full-screen, by adding the following line
# as an XML attribute to the manifest's <application> tag :
#     android:theme="@android:style/Theme.DeviceDefault.NoActionBar.Fullscreen
# Defaults to false.
fullscreen = false

# The maximum supported OpenGL ES version , as claimed by the manifest.
# Defaults to 2.0.
# See https://developer.android.com/guide/topics/graphics/opengl.html#manifest
opengles_version_major = 3
opengles_version_minor = 2

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

# Adds a uses-feature element to the manifest
# Supported keys: name, required, version
# The glEsVersion attribute is not supported using this section. 
# It can be specified using the opengles_version_major and opengles_version_minor values
# See https://developer.android.com/guide/topics/manifest/uses-feature-element
[[package.metadata.android.feature]]
name = "android.hardware.camera"

[[package.metadata.android.feature]]
name = "android.hardware.vulkan.level"
version = "1"
required = false

# Adds a uses-permission element to the manifest.
# Note that android_version 23 and higher, Android requires the application to request permissions at runtime.
# There is currently no way to do this using a pure NDK based application.
# See https://developer.android.com/guide/topics/manifest/uses-permission-element
[[package.metadata.android.permission]]
name = "android.permission.WRITE_EXTERNAL_STORAGE"
max_sdk_version = 18

[[package.metadata.android.permission]]
name = "android.permission.CAMERA"
```
