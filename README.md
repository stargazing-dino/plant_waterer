# Plant Waterer on the Microbit-v2

Add the target:

```bash
rustup target add thumbv7em-none-eabihf
```

./.cargo/config.toml

```toml
[build]
target = "thumbv7em-none-eabihf"

[target.thumbv7em-none-eabihf]
rustflags = ["-C", "link-arg=-Tlink.x", "-C", "link-arg=-Tdefmt.x"]

[target.'cfg(all(target_arch = "arm", target_os = "none"))']
runner = "probe-rs run --chip nRF52833_xxAA"
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
microbit-bsp = "0.3.0"
embassy-futures = "0.1.1"
```

## Nice to haves

Non-volatile memory controller
