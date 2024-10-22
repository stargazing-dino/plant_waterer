#![no_std]
#![no_main]

use crate::embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_executor::Spawner;
use microbit_bsp::*;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let board = Microbit::new(Default::default());

    // Use button A to control the pump
    let mut button = board.btn_a;

    // Use P0 as the output pin to control the MOSFET
    let mut pump_control = Output::new(board.p1, Level::Low, OutputDrive::Standard);

    loop {
        button.wait_for_low().await;

        defmt::info!("Watering plant...");
        pump_control.set_high();

        button.wait_for_high().await;

        pump_control.set_low();
        defmt::info!("Watering complete.");
    }
}
