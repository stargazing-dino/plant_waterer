#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::Timer;
use microbit_bsp::*;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    loop {
        defmt::info!("Hello World!");
        Timer::after_millis(1000).await;
    }
}
