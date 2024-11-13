#![no_std]
#![no_main]

use ble::{softdevice_task, Server};
use defmt::unwrap;
use embassy_executor::Spawner;
use embassy_time::Timer;
use microbit_bsp::embassy_nrf;
use nrf_softdevice::Softdevice;
use {defmt_rtt as _, panic_probe as _};

mod ble;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let _ = embassy_nrf::init(Default::default());

    let softdevice_config = nrf_softdevice::Config::default();
    let softdevice = Softdevice::enable(&softdevice_config);
    let server = unwrap!(Server::new(softdevice));

    unwrap!(spawner.spawn(softdevice_task(softdevice)));

    loop {
        defmt::info!("Hello World!");
        Timer::after_millis(1000).await;
    }
}
