#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_time::Duration;
use microbit_bsp::{
    embassy_nrf::gpio::{AnyPin, Input, Level, Output, OutputDrive},
    embassy_nrf::{
        bind_interrupts, saadc,
        saadc::{ChannelConfig, Config, Saadc},
    },
    *,
};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    SAADC => saadc::InterruptHandler;
});

const MEASUREMENT_INTERVAL: Duration = Duration::from_secs(5);

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let mut board = Microbit::new(Default::default());

    // Use button A to control the pump
    let mut button = board.btn_a;

    // Use P0 as the output pin to control the MOSFET
    let mut pump_control = Output::new(board.p1, Level::Low, OutputDrive::Standard);

    // Setup for the SAADC peripheral
    let mut config = Config::default();
    config.resolution = saadc::Resolution::_12BIT;
    let channel_config = ChannelConfig::single_ended(&mut board.p2);
    let mut saadc = Saadc::new(board.saadc, Irqs, config, [channel_config]);

    // Calibrate by taking a reading when the sensor is in the air (very dry)
    let dry_reading = calibrate_sensor(&mut saadc, &mut button).await;

    // We want something a little less than the dry reading to trigger watering
    let offset = 200;
    let dry_reading = dry_reading - offset;

    loop {
        let button_press = button.wait_for_low();
        let measurement_interval = embassy_time::Timer::after(MEASUREMENT_INTERVAL);

        // wait for either the measurement interval to pass or the button to be pressed
        match select(button_press, measurement_interval).await {
            Either::First(_) => {
                defmt::info!("Manual watering requested");
                pump_control.set_high();
                button.wait_for_high().await;
                pump_control.set_low();
                defmt::info!("Watering complete.");
                continue;
            }
            Either::Second(_) => {
                defmt::info!("Taking moisture reading");

                let reading = read_moisture(&mut saadc).await;

                defmt::info!("Moisture reading: {}", reading);

                if reading > dry_reading {
                    defmt::info!("Soil is dry, watering");
                    pump_control.set_high();
                    embassy_time::Timer::after(Duration::from_secs(5)).await;
                    pump_control.set_low();
                    defmt::info!("Watering complete.");
                } else {
                    defmt::info!("Soil is moist, no watering needed");
                }
            }
        }
    }
}

async fn calibrate_sensor(adc: &mut Saadc<'_, 1>, button: &mut Input<'static, AnyPin>) -> i16 {
    defmt::info!("Place sensor in dry soil and press button A");

    button.wait_for_low().await;
    let reading = read_moisture(adc).await;
    button.wait_for_high().await;

    defmt::info!("Dry reading: {}", reading);
    reading
}

/// For this particular sensor:
/// - ~2840: Very dry (in air/dry soil)
/// - ~1180: Very wet (submerged in water)
/// Lower numbers indicate more moisture
async fn read_moisture(adc: &mut Saadc<'_, 1>) -> i16 {
    let mut buf = [0i16; 1];
    adc.sample(&mut buf).await;
    buf[0]
}
