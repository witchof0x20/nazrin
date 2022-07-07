use color_eyre::eyre::{self, WrapErr};
use ddc_hi::{Ddc, Display};
use futures_util::future::ready;
use futures_util::stream::StreamExt;
use std::ffi::OsStr;
use std::sync::{Arc, Mutex};
use tokio_udev::{EventType, MonitorBuilder};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Set up eyre
    color_eyre::install()?;
    let mut my_display = None;
    for mut display in Display::enumerate() {
        if let Ok(()) = display.update_capabilities() {
            println!(
                "{:?} {}: {:?} {:?}",
                display.info.backend,
                display.info.id,
                display.info.manufacturer_id,
                display.info.model_name
            );
            my_display = Some(display)
        }
    }
    // If the display is not found. fail out
    let mut display = my_display.ok_or_else(|| eyre::eyre!("Failed to find a display"))?;
    const SOURCE: u8 = 0x60;
    if let Some(feature) = display.info.mccs_database.get(SOURCE) {
        display
            .handle
            .get_vcp_feature(feature.code)
            .map(|_| ())
            .map_err(|err| {
                eyre::eyre!("Display does not support setting source over ddc: {}", err)
            })?;
    }
    let display = Arc::new(Mutex::new(display));
    println!("Finished enumerating displays");

    let builder = MonitorBuilder::new()
        .wrap_err("Failed to create udev monitor builder")?
        .match_subsystem_devtype("usb", "usb_device")
        .wrap_err("Failed to add usb filter")?;
    let target_product = "46d/c332/301";
    let monitor = builder.listen().wrap_err("Couldn't create MonitorSocket")?;

    monitor
        .for_each(|event| {
            // Add, then bind, so treat Bind as the proper event
            match event.event_type() {
                EventType::Bind => {
                    if let Some(product) = event
                        .device()
                        .property_value("PRODUCT")
                        .and_then(OsStr::to_str)
                    {
                        if product == target_product {
                            println!("Device detected, swapping the monitor to us");
                            // Get reference to the display mutex
                            let display = display.clone();
                            if let Ok(mut display) = display.lock() {
                                display.handle.set_vcp_feature(SOURCE, 0x0F).unwrap();
                            };
                        }
                    }
                }
                EventType::Remove => {
                    if let Some(product) = event
                        .device()
                        .property_value("PRODUCT")
                        .and_then(OsStr::to_str)
                    {
                        if product == target_product {
                            println!("Device removed, swapping the monitor away from us");
                            // Get reference to the display mutex
                            let display = display.clone();
                            if let Ok(mut display) = display.lock() {
                                display.handle.set_vcp_feature(SOURCE, 0x12).unwrap();
                            };
                        }
                    }
                }
                _ => {}
            }
            ready(())
        })
        .await;
    Ok(())
}
