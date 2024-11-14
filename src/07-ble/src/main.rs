#![no_std]
#![no_main]

use ble::{softdevice_task, PlantServiceEvent, Server, ServerEvent, ADV_DATA, SCAN_DATA};
use defmt::unwrap;
use embassy_executor::Spawner;
use microbit_bsp::embassy_nrf;
use nrf_softdevice::{
    ble::{gatt_server, peripheral},
    Softdevice,
};
use {defmt_rtt as _, panic_probe as _};

mod ble;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let _ = embassy_nrf::init(Default::default());
    let softdevice_config = nrf_softdevice::Config::default();
    let softdevice = Softdevice::enable(&softdevice_config);
    let server = unwrap!(Server::new(softdevice));

    unwrap!(spawner.spawn(softdevice_task(softdevice)));

    let config = peripheral::Config::default();
    let connectable_advertisement = peripheral::ConnectableAdvertisement::ScannableUndirected {
        adv_data: &ADV_DATA,
        scan_data: &SCAN_DATA,
    };

    loop {
        let connection =
            match peripheral::advertise_connectable(softdevice, connectable_advertisement, &config)
                .await
            {
                Ok(connection) => connection,
                Err(error) => {
                    defmt::info!("Failed to establish connection: {:?}", error);
                    continue;
                }
            };

        defmt::info!("Connection established");

        let disconnected_error =
            gatt_server::run(&connection, &server, |server_event| match server_event {
                ServerEvent::PlantService(event) => match event {
                    PlantServiceEvent::PumpControlWrite(value) => {
                        defmt::info!("Pump control set to: {}", value);
                    }
                    PlantServiceEvent::MoistureLevelCccdWrite { notifications } => {
                        defmt::info!("Moisture level notifications: {}", notifications);
                    }
                },
            })
            .await;

        defmt::info!(
            "gatt_server run exited with error: {:?}",
            disconnected_error
        );
    }
}
