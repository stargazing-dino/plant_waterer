#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_nrf::gpio::{Input, Level, Output, OutputDrive, Pin as _, Pull};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());

    // Use button A to control the pump
    let mut button = Input::new(p.P0_14.degrade(), Pull::Up);

    // Use P0 as the output pin to control the MOSFET
    let mut pump_control = Output::new(p.P0_02, Level::Low, OutputDrive::Standard);

    loop {
        button.wait_for_low().await;

        defmt::info!("Watering plant...");
        pump_control.set_high();

        button.wait_for_high().await;

        pump_control.set_low();
        defmt::info!("Watering complete.");
    }
}
