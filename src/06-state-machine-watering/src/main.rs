#![no_std]
#![no_main]

use debouncer::Debouncer;
use embassy_executor::Spawner;
use embassy_nrf::{
    bind_interrupts,
    gpio::{Input, Level, Output, OutputDrive, Pin, Pull},
    saadc::{self, ChannelConfig, Config, Saadc},
};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, channel::Channel};
use embassy_time::Duration;

mod debouncer;

use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    SAADC => saadc::InterruptHandler;
});

const MEASUREMENT_INTERVAL: Duration = Duration::from_secs(10);
const WATERING_DURATION: Duration = Duration::from_secs(5);
const DEFAULT_THRESHOLD: i16 = 2000;
const THRESHOLD_BUFFER: i16 = 100;

// Transitions
#[derive(Debug, PartialEq)]
enum Event {
    Water,
    WateringComplete,
    Measure,
    Calibrate,
}

#[derive(Debug, PartialEq)]
enum SystemState {
    Watering,
    Idle,
}

static CHANNEL: Channel<ThreadModeRawMutex, Event, 1> = Channel::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut p = embassy_nrf::init(Default::default());

    // Setup hardware
    let button_a = Input::new(p.P0_14.degrade(), Pull::Up);
    let button_a = Debouncer::new(button_a, Duration::from_millis(20));

    let button_b = Input::new(p.P0_23.degrade(), Pull::Up);
    let button_b = Debouncer::new(button_b, Duration::from_millis(20));

    let pump_control = Output::new(p.P0_03.degrade(), Level::Low, OutputDrive::Standard);

    // Setup for the SAADC peripheral
    let mut config = Config::default();
    config.resolution = saadc::Resolution::_12BIT;
    let channel_config = ChannelConfig::single_ended(&mut p.P0_04);
    let saadc = Saadc::new(p.SAADC, Irqs, config, [channel_config]);

    // Spawn our tasks
    spawner.spawn(button_a_task(button_a)).unwrap();
    spawner.spawn(button_b_task(button_b)).unwrap();
    spawner.spawn(measurement_task()).unwrap();
    spawner.spawn(control_task(pump_control, saadc)).unwrap();
}

#[embassy_executor::task]
async fn button_a_task(mut button: Debouncer<'static>) {
    let sender = CHANNEL.sender();
    loop {
        button.debounce().await;
        sender.send(Event::Water).await;
        button.debounce().await;
        sender.send(Event::WateringComplete).await;
    }
}

#[embassy_executor::task]
async fn button_b_task(mut button: Debouncer<'static>) {
    let sender = CHANNEL.sender();
    loop {
        button.debounce().await;
        sender.send(Event::Calibrate).await;
    }
}

#[embassy_executor::task]
async fn measurement_task() {
    let sender = CHANNEL.sender();
    loop {
        embassy_time::Timer::after(MEASUREMENT_INTERVAL).await;
        sender.send(Event::Measure).await;
    }
}

#[embassy_executor::task]
async fn control_task(mut pump_control: Output<'static>, mut saadc: Saadc<'static, 1>) {
    defmt::info!("System started. Press button A to manually water.");
    defmt::info!("System started. Press button B to calibrate.");

    let receiver = CHANNEL.receiver();

    // Start with a default threshold
    let mut moisture_threshold = DEFAULT_THRESHOLD;
    let mut system_state = SystemState::Idle;

    // https://www.youtube.com/watch?v=z-0-bbc80JM
    loop {
        let event = receiver.receive().await;

        system_state = match (system_state, event) {
            // Handle watering state transitions
            (SystemState::Idle, Event::Water) => {
                defmt::info!("Watering requested");
                pump_control.set_high();
                SystemState::Watering
            }

            (SystemState::Watering, Event::WateringComplete) => {
                defmt::info!("Watering complete");
                pump_control.set_low();
                SystemState::Idle
            }

            // Handle moisture measurement
            (SystemState::Idle, Event::Measure) => {
                defmt::info!("Taking moisture reading");
                let reading = read_moisture(&mut saadc).await;
                defmt::info!("Moisture reading: {}", reading);

                if reading > moisture_threshold {
                    defmt::info!("Soil is dry, watering");
                    pump_control.set_high();
                    embassy_time::Timer::after(WATERING_DURATION).await;
                    pump_control.set_low();
                    defmt::info!("Automatic watering complete");
                }

                SystemState::Idle
            }

            // Handle calibration
            (SystemState::Idle, Event::Calibrate) => {
                defmt::info!("Starting calibration...");
                defmt::info!("Taking dry soil reading");
                let dry_reading = read_moisture(&mut saadc).await;
                defmt::info!("Dry reading: {}", dry_reading);

                moisture_threshold = dry_reading - THRESHOLD_BUFFER;
                defmt::info!(
                    "Calibration complete. New threshold: {}",
                    moisture_threshold
                );
                SystemState::Idle
            }

            // Ignore any other state/event combinations
            (current_state, _) => current_state,
        };
    }
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
