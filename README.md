# Plant Waterer on the Microbit-v2

Add the target:

```bash
rustup target add thumbv7em-none-eabi
```

./.cargo/config.toml

```toml
[target.'cfg(all(target_arch = "arm", target_os = "none"))']
# replace nRF82840_xxAA with your chip as listed in `probe-rs chip list`
runner = "probe-rs run --chip nRF52840_xxAA"

[build]
target = "thumbv7em-none-eabi"

[env]
DEFMT_LOG = "trace"
```

./build.rs

```rs
//! This build script copies the `memory.x` file from the crate root into
//! a directory where the linker can always find it at build time.
//! For many projects this is optional, as the linker always searches the
//! project root directory -- wherever `Cargo.toml` is. However, if you
//! are using a workspace or have a more complicated build setup, this
//! build script becomes required. Additionally, by requesting that
//! Cargo re-run the build script whenever `memory.x` is changed,
//! updating `memory.x` ensures a rebuild of the application with the
//! new memory settings.

use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    // Put `memory.x` in our output directory and ensure it's
    // on the linker search path.
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("memory.x"))
        .unwrap();
    println!("cargo:rustc-link-search={}", out.display());

    // By default, Cargo will re-run a build script whenever
    // any file in the project changes. By specifying `memory.x`
    // here, we ensure the build script is only re-run when
    // `memory.x` is changed.
    println!("cargo:rerun-if-changed=memory.x");

    println!("cargo:rustc-link-arg-bins=--nmagic");
    println!("cargo:rustc-link-arg-bins=-Tlink.x");
    println!("cargo:rustc-link-arg-bins=-Tdefmt.x");
}
```

./memory.x

```x
MEMORY
{
  /* NOTE 1 K = 1 KiBi = 1024 bytes */
  /* These values correspond to the NRF52833 */
  FLASH : ORIGIN = 0x00000000, LENGTH = 512K
  RAM : ORIGIN = 0x20000000, LENGTH = 128K
}
```

Add this so we get rid of the error:

./.vscode/settings.json

```json
{
    "rust-analyzer.check.allTargets": false,
    "rust-analyzer.cargo.target": "thumbv7em-none-eabihf",
}
```

Dependencies:

```toml
cortex-m = { version = "0.7.7", features = ["critical-section-single-core"] }
cortex-m-rt = "0.7.3"
panic-probe = { version = "0.3", features = ["print-defmt"] }
defmt-rtt = "0.4.1"
defmt = "0.3.8"
embedded-hal = "1.0.0"
embassy-nrf = { version = "0.2.0", features = [
    "defmt",
    "nrf52833",
    "time-driver-rtc1",
    "gpiote",
    "unstable-pac",
    "time",
] }
embassy-executor = { version = "0.6.0", features = [
    "task-arena-size-4096",
    "arch-cortex-m",
    "executor-thread",
    "defmt",
    "integrated-timers",
    "executor-interrupt",
] }
embassy-time = { version = "0.3.2", features = [
    "defmt",
    "defmt-timestamp-uptime",
] }

```
