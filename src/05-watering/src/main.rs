#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_nrf::gpio::{Input, Level, Output, OutputDrive, Pin as _, Pull};
use embassy_nrf::{
    bind_interrupts, saadc,
    saadc::{ChannelConfig, Config, Saadc},
};
use embassy_time::Duration;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    SAADC => saadc::InterruptHandler;
});

const MEASUREMENT_INTERVAL: Duration = Duration::from_secs(10);
const WATERING_DURATION: Duration = Duration::from_secs(5);

const THRESHOLD_BUFFER: i16 = 100;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let mut p = embassy_nrf::init(Default::default());
    let mut button = Input::new(p.P0_14.degrade(), Pull::Up);
    let mut pump_control = Output::new(p.P0_03, Level::Low, OutputDrive::Standard);

    // Setup for the SAADC peripheral
    let mut config = Config::default();
    config.resolution = saadc::Resolution::_12BIT;
    let channel_config = ChannelConfig::single_ended(&mut p.P0_04);
    let mut saadc = Saadc::new(p.SAADC, Irqs, config, [channel_config]);

    // Calibrate by taking a reading when the sensor is in the air (very dry)
    let dry_reading = calibrate_sensor(&mut saadc, &mut button).await;

    // We want something a little less than the dry reading to trigger watering
    let moisture_threshold = dry_reading - THRESHOLD_BUFFER;

    loop {
        let button_press = button.wait_for_low();
        let measurement_interval = embassy_time::Timer::after(MEASUREMENT_INTERVAL);

        match select(button_press, measurement_interval).await {
            Either::First(_) => handle_manual_watering(&mut button, &mut pump_control).await,
            Either::Second(_) => {
                handle_auto_watering(&mut saadc, &mut pump_control, moisture_threshold).await
            }
        }
    }
}

async fn handle_manual_watering(button: &mut Input<'static>, pump: &mut Output<'_>) {
    defmt::info!("Manual watering requested");
    pump.set_high();
    button.wait_for_high().await;
    pump.set_low();
    defmt::info!("Watering complete.");
}

async fn handle_auto_watering(saadc: &mut Saadc<'_, 1>, pump: &mut Output<'_>, threshold: i16) {
    defmt::info!("Taking moisture reading");
    let reading = read_moisture(saadc).await;
    defmt::info!("Moisture reading: {}", reading);

    if reading > threshold {
        defmt::info!("Soil is dry, watering");
        pump.set_high();
        embassy_time::Timer::after(WATERING_DURATION).await;
        pump.set_low();
        defmt::info!("Watering complete.");
    } else {
        defmt::info!("Soil is moist, no watering needed");
    }
}

async fn calibrate_sensor(adc: &mut Saadc<'_, 1>, button: &mut Input<'static>) -> i16 {
    defmt::info!("Place sensor in dry soil and press button A");

    button.wait_for_low().await;
    let reading = read_moisture(adc).await;
    button.wait_for_high().await;

    defmt::info!("Dry reading: {}", reading);
    reading
}

/// For this particular sensor:
/// Lower numbers indicate more moisture
/// - ~2840: Very dry (in air/dry soil)
/// - ~1180: Very wet (submerged in water)
async fn read_moisture(adc: &mut Saadc<'_, 1>) -> i16 {
    let mut buf = [0i16; 1];
    adc.sample(&mut buf).await;
    buf[0]
}
