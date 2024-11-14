probe-rs erase --chip nrf52840_xxAA --allow-erase-all 
probe-rs download --verify --format hex --chip nRF52840_xxAA s140_nrf52_7.3.0/s140_nrf52_7.3.0_softdevice.hex
cargo run