#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_futures::select::{select3, Either3};
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
        let button_a_future = btn_a.wait_for_low();
        let button_b_future = btn_b.wait_for_low();

        match select3(scroll_future, button_a_future, button_b_future).await {
            Either3::First(_) => {
                defmt::info!("Scroll completed, restarting...");
            }
            Either3::Second(_) => {
                defmt::info!("A pressed");
                display
                    .display(display::fonts::ARROW_LEFT, Duration::from_secs(1))
                    .await;
            }
            Either3::Third(_) => {
                defmt::info!("B pressed");
                display
                    .display(display::fonts::ARROW_RIGHT, Duration::from_secs(1))
                    .await;
            }
        }
    }
}
