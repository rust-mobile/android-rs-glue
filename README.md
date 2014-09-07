# android_glue

```toml
[dependencies.android_glue]
git = "https://github.com/tomaka/android-rs-glue"
```

## Usage

```rust
#[phase(plugin, link)] 
extern crate android_glue;

android_start!(main)

fn main() {
    // ...
}
```

## How to compile

Preliminary steps:

 - If you are on Linux 64 bits, install the 32 bits binaries: `apt-get install libc6-i386 lib32z1 lib32stdc++6`

 - Download and unzip [the Android NDK](http://developer.android.com/tools/sdk/ndk/index.html)
 - Generate a stand-alone toolchain of the NDK, example: `./android-ndk-r10/build/tools/make-standalone-toolchain.sh --platform=android-L --toolchain=arm-linux-androideabi --install-dir=/opt/ndk_standalone --ndk-dir=/home/you/android-ndk-r10`

 - Clone the Rust compiler: `git clone https://github.com/rust-lang/rust.git`
 - Compile Rust for Android: `mkdir rust-build`, `cd rust-build`, `../rust/configure --target=arm-linux-androideabi --android-cross-path=/opt/ndk_standalone`, `make`, `make install`

 - Download and unzip [Cargo](https://github.com/rust-lang/cargo#installing-cargo-from-nightlies)
 - Install Cargo: `./cargo-nightly-x86_64-unknown-linux-gnu/install.sh`

 - Install the Java JDK and Ant: `apt-get install openjdk-7-jdk ant`

 - Download and unzip [the Android SDK](http://developer.android.com/sdk/index.html) (under *SDK tools* in *VIEW ALL DOWNLOADS AND SIZES*)
 - Update the SDK: `./android-sdk-linux/tools/android update sdk -u`

Building:
 - In `apk-builder`: `cargo build`
 - In `glue`: `cargo build --target=arm-linux-androideabi`
 - `rustc examples/basic.rs -C linker=/opt/ndk_standalone/bin/arm-linux-androideabi-gcc -C link_args="-Wl,-E" --crate-name example --crate-type bin -o example --target arm-linux-androideabi -L glue/target/arm-linux-androideabi`
 - `/opt/ndk_standalone/bin/arm-linux-androideabi-elfedit --output-type dyn example`
 - `apk-builder/target/apk-builder --sdk /home/user/android-sdk-linux -o example.apk example`
