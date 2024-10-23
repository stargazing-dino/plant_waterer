#![no_std]
#![no_main]

use crate::embassy_nrf::gpio::{Level, Output, OutputDrive};
use crate::embassy_nrf::pwm::SimplePwm;
use embassy_executor::Spawner;
use microbit_bsp::*;
use speaker::{NamedPitch, Pitch, PwmSpeaker};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let board = Microbit::new(Default::default());

    // Use button A to control the pump
    let mut button = board.btn_a;

    // Use P0 as the output pin to control the MOSFET
    let mut pump_control = Output::new(board.p1, Level::Low, OutputDrive::Standard);

    let mut speaker = PwmSpeaker::new(SimplePwm::new_1ch(board.pwm0, board.speaker));

    loop {
        button.wait_for_low().await;

        defmt::info!("Watering plant...");
        pump_control.set_high();

        // Start playing a note
        speaker.start_note(Pitch::Named(NamedPitch::A1));

        button.wait_for_high().await;

        // Stop the sound and pump
        speaker.stop();
        pump_control.set_low();
        defmt::info!("Watering complete.");
    }
}
