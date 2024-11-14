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
pub struct KeyValueService {
    #[characteristic(
        uuid = "12345678-1234-5678-1234-56789abcdef1",
        write,
        write_without_response,
        read,
        notify
    )]
    pub command: [u8; 512],
}

#[nrf_softdevice::gatt_server]
pub struct Server {
    pub key_value_service: KeyValueService,
}

#[embassy_executor::task]
pub async fn softdevice_task(sd: &'static Softdevice) -> ! {
    sd.run().await
}

#[embassy_executor::task]
pub async fn ble_server_task(
    softdevice: &'static Softdevice,
    server: Server,
    connectable_advertisement: peripheral::ConnectableAdvertisement<'static>,
    config: peripheral::Config,
) {
    loop {
        // Start advertising and wait for a connection
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

        // Run the GATT server
        let disconnected_error =
            gatt_server::run(&connection, &server, |server_event| match server_event {
                ServerEvent::KeyValueService(event) => match event {
                    KeyValueServiceEvent::CommandWrite(data) => {
                        if let Some(response) = handle_command(&data) {
                            // Send the response as a notification
                            if let Err(e) = server
                                .key_value_service
                                .command_notify(&connection, &pad_value::<512>(&response))
                            {
                                defmt::info!("Failed to send response: {:?}", e);
                            }
                        }
                    }
                    _ => {}
                },
            })
            .await;

        defmt::info!(
            "gatt_server run exited with error: {:?}",
            disconnected_error
        );
    }
}
