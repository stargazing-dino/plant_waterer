#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_time::Duration;
use microbit_bsp::*;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let board = Microbit::default();

    let mut display = board.display;
    let mut btn_a = board.btn_a;
    let mut btn_b = board.btn_b;

    display.set_brightness(display::Brightness::MAX);
    defmt::info!("Application started, press buttons!");

    loop {
        let scroll_future = display.scroll("Hello, World!");

        match select(
            scroll_future,
            select(btn_a.wait_for_low(), btn_b.wait_for_low()),
        )
        .await
        {
            Either::First(_) => {
                // Scroll completed, restart it
                defmt::info!("Scroll completed, restarting...");
            }
            Either::Second(Either::First(_)) => {
                defmt::info!("A pressed");
                display
                    .display(display::fonts::ARROW_LEFT, Duration::from_secs(1))
                    .await;
            }
            Either::Second(Either::Second(_)) => {
                defmt::info!("B pressed");
                display
                    .display(display::fonts::ARROW_RIGHT, Duration::from_secs(1))
                    .await;
            }
        }
    }
}
