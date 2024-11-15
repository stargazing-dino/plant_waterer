#![no_std]
#![no_main]

extern crate alloc;
use core::cell::RefCell;

use ble::{softdevice_task, PlantServiceEvent, Server, ServerEvent, ADV_DATA, SCAN_DATA};
use defmt::unwrap;
use embassy_executor::Spawner;
use embassy_nrf::{
    bind_interrupts,
    gpio::{Input, Level, Output, OutputDrive, Pin as _, Pull},
    saadc::{self, ChannelConfig, Config, Saadc},
};
use embassy_sync::{
    blocking_mutex::raw::{CriticalSectionRawMutex, ThreadModeRawMutex},
    channel::Channel,
    mutex::Mutex,
    signal::Signal,
};
use embassy_time::Duration;
use nrf_softdevice::{
    ble::{gatt_server, peripheral, Connection},
    Softdevice,
};
use {defmt_rtt as _, panic_probe as _};

mod ble;
mod debouncer;

bind_interrupts!(struct Irqs {
    SAADC => saadc::InterruptHandler;
});

const MEASUREMENT_INTERVAL: Duration = Duration::from_secs(10);
const WATERING_DURATION: Duration = Duration::from_secs(5);
const DEFAULT_THRESHOLD: u16 = 2000;
const THRESHOLD_BUFFER: u16 = 100;

enum ConnectionState {
    Connected(Connection),
    Disconnected,
}

static SERVER: Mutex<CriticalSectionRawMutex, RefCell<Option<Server>>> =
    Mutex::new(RefCell::new(None));

static CONNECTION_STATE: Signal<ThreadModeRawMutex, ConnectionState> = Signal::new();

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

static CHANNEL: Channel<ThreadModeRawMutex, Event, 4> = Channel::new();
static MOISTURE_SIGNAL: Signal<ThreadModeRawMutex, u16> = Signal::new();

#[embassy_executor::task]
async fn button_task(mut button: debouncer::Debouncer<'static>) {
    let sender = CHANNEL.sender();
    loop {
        button.debounce().await;
        sender.send(Event::Water).await;
        button.debounce().await;
        sender.send(Event::WateringComplete).await;
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
async fn ble_task(softdevice: &'static Softdevice) {
    let config = peripheral::Config::default();
    let sender = CHANNEL.sender();

    loop {
        let connection = match peripheral::advertise_connectable(
            softdevice,
            peripheral::ConnectableAdvertisement::ScannableUndirected {
                adv_data: &ADV_DATA,
                scan_data: &SCAN_DATA,
            },
            &config,
        )
        .await
        {
            Ok(connection) => connection,
            Err(error) => {
                defmt::warn!("Advertisement error: {:?}", error);
                continue;
            }
        };

        defmt::info!("Connection established");
        CONNECTION_STATE.signal(ConnectionState::Connected(connection.clone()));
        let server_guard = SERVER.lock().await;
        let server_ref = server_guard.borrow();

        if let Some(ref server) = server_ref.as_ref() {
            let _disconnected = gatt_server::run(&connection, *server, |event| match event {
                ServerEvent::PlantService(evt) => match evt {
                    PlantServiceEvent::PumpControlWrite(value) => {
                        if value > 0 {
                            sender.send(Event::Water);
                        } else {
                            sender.send(Event::WateringComplete);
                        }
                    }
                    PlantServiceEvent::MoistureLevelCccdWrite { notifications: _ } => {}
                },
            })
            .await;
        }

        CONNECTION_STATE.signal(ConnectionState::Disconnected);
        defmt::info!("Disconnected");
    }
}

#[embassy_executor::task]
async fn control_task(mut pump_control: Output<'static>, mut saadc: Saadc<'static, 1>) {
    let receiver = CHANNEL.receiver();
    let moisture_threshold = DEFAULT_THRESHOLD;
    let mut system_state = SystemState::Idle;

    loop {
        let event = receiver.receive().await;

        system_state = match (system_state, event) {
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
            (SystemState::Idle, Event::Measure) => {
                let reading = read_moisture(&mut saadc).await;
                defmt::info!("Moisture reading: {}", reading);

                // Update BLE characteristic if connected
                if let ConnectionState::Connected(ref connection) = CONNECTION_STATE.wait().await {
                    let server_guard = SERVER.lock().await;
                    let mut server_ref = server_guard.borrow_mut();

                    if let Some(ref mut server) = server_ref.as_mut() {
                        server
                            .plant_service
                            .moisture_level_notify(connection, &reading)
                            .unwrap();
                    }
                }
                MOISTURE_SIGNAL.signal(reading);

                if reading > moisture_threshold {
                    defmt::info!("Soil is dry, watering");
                    pump_control.set_high();
                    embassy_time::Timer::after(WATERING_DURATION).await;
                    pump_control.set_low();
                    defmt::info!("Automatic watering complete");
                }
                SystemState::Idle
            }
            (current_state, _) => current_state,
        };
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());

    // Initialize softdevice
    let softdevice_config = nrf_softdevice::Config::default();
    let softdevice = Softdevice::enable(&softdevice_config);

    // set global SERVER
    SERVER
        .lock()
        .await
        .replace(Some(unwrap!(Server::new(softdevice))));

    // Initialize hardware
    let button = Input::new(p.P0_14.degrade(), Pull::Up);
    let button = debouncer::Debouncer::new(button, Duration::from_millis(20));

    let pump_control = Output::new(p.P0_03.degrade(), Level::Low, OutputDrive::Standard);

    // Setup SAADC
    let mut config = Config::default();
    config.resolution = saadc::Resolution::_12BIT;
    let channel_config = ChannelConfig::single_ended(p.P0_04);
    let saadc = Saadc::new(p.SAADC, Irqs, config, [channel_config]);

    // Spawn tasks
    unwrap!(spawner.spawn(softdevice_task(softdevice)));
    unwrap!(spawner.spawn(ble_task(softdevice)));
    unwrap!(spawner.spawn(button_task(button)));
    unwrap!(spawner.spawn(measurement_task()));
    unwrap!(spawner.spawn(control_task(pump_control, saadc)));
}

async fn read_moisture(adc: &mut Saadc<'_, 1>) -> u16 {
    let mut buf = [0i16; 1];
    adc.sample(&mut buf).await;
    buf[0] as u16
}
