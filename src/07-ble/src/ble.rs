use nrf_softdevice::{
    ble::{
        advertisement_builder::{
            Flag, LegacyAdvertisementBuilder, LegacyAdvertisementPayload, ServiceList,
        },
        gatt_server, peripheral,
    },
    Softdevice,
};

const DEVICE_NAME: &str = "planty";

pub static ADV_DATA: LegacyAdvertisementPayload = LegacyAdvertisementBuilder::new()
    .flags(&[Flag::GeneralDiscovery, Flag::LE_Only])
    .full_name(DEVICE_NAME)
    .build();
pub static SCAN_DATA: LegacyAdvertisementPayload = LegacyAdvertisementBuilder::new()
    .services_128(
        ServiceList::Complete,
        &[0x12345678_1234_5678_1234_56789abcdef0_u128.to_le_bytes()],
    )
    .build();

#[nrf_softdevice::gatt_service(uuid = "12345678-1234-5678-1234-56789abcdef0")]
pub struct PlantService {
    #[characteristic(
        uuid = "12345678-1234-5678-1234-56789abcdef1",
        write,
        write_without_response
    )]
    pub pump_control: u8,

    #[characteristic(uuid = "12345678-1234-5678-1234-56789abcdef2", read, notify)]
    pub moisture_level: u16,
}

#[nrf_softdevice::gatt_server]
pub struct Server {
    pub plant_service: PlantService,
}

#[embassy_executor::task]
pub async fn softdevice_task(sd: &'static Softdevice) -> ! {
    sd.run().await
}

#[embassy_executor::task]
pub async fn ble_server_task(
    softdevice: &'static Softdevice,
    server_handle: &'static ServerHandle,
    connectable_advertisement: peripheral::ConnectableAdvertisement<'static>,
    config: peripheral::Config,
) {
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
            gatt_server::run(&connection, &server_handle.server, |server_event| {
                match server_event {
                    ServerEvent::PlantService(event) => match event {
                        PlantServiceEvent::PumpControlWrite(value) => {
                            // Turn pump on if value is non-zero
                            (server_handle.pump_callback)(value != 0);
                            defmt::info!("Pump control set to: {}", value);
                        }
                        PlantServiceEvent::MoistureLevelCccdWrite { notifications } => {
                            defmt::info!("Moisture level notifications: {}", notifications);
                        }
                    },
                }
            })
            .await;

        defmt::info!(
            "gatt_server run exited with error: {:?}",
            disconnected_error
        );
    }
}
