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
