// See the "macOS permissions note" in README.md before running this on macOS
// Big Sur or later.

use btleplug::api::{Central, Characteristic, Manager as _, Peripheral, ScanFilter};
use btleplug::platform::Manager;
extern crate derive_more;
use derive_more::{Add, Display, Error as DError, From, Into};
use futures::StreamExt;
use std::error::Error;
use std::path::PathBuf;
use std::pin::Pin;
use std::time::Duration;
use structopt::StructOpt;
use tokio::time;

mod controller;

use controller::controller::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();

    let manager = Manager::new().await?;
    let adapter_list = manager.adapters().await?;
    if adapter_list.is_empty() {
        eprintln!("No Bluetooth adapters found");
    }

    let adapter = adapter_list[0].clone();

    println!("Starting scan on {}...", adapter.adapter_info().await?);
    adapter
        .start_scan(ScanFilter::default())
        .await
        .expect("Can't scan BLE adapter for connected devices...");

    std::thread::sleep(time::Duration::from_secs(1));
    let peripherals = adapter.peripherals().await?;
    if peripherals.is_empty() {
        eprintln!("->>> BLE peripheral devices were not found, sorry. Exiting...");
    } else {
        let x = async {
            let mut res: Option<_> = Option::None;
            for peripheral in peripherals.iter() {
                let properties = peripheral.properties().await;
                match properties {
                    Ok(prop) => {
                        let local_name = prop
                            .unwrap()
                            .local_name
                            .unwrap_or(String::from("(peripheral name unknown)"));
                        if local_name.to_lowercase().contains("walkingpad") {
                            res = Option::Some(peripheral.clone());
                            break;
                        }
                    }
                    Err(e) => {
                        println!("ERROR {}", e);
                    }
                }
            }
            res
        };

        let walkingpad = x.await;

        if let Some(walkingpad) = walkingpad {
            let pad = Pad::new(&walkingpad).await?;

            pad.services().await;

            pad.subs().await?;

            let k = pad.gets().await?;
            tokio::spawn(async move {
                k.for_each(|data| async move {
                    println!("Received data [{:?}]: {:?}", data.uuid, data.value)
                })
                .await
            });

            pad.ask_stats().await?;
            std::thread::sleep(Duration::from_secs(5));
            pad.ask_profile().await?;

            std::thread::sleep(time::Duration::from_secs(5));

            // pad.switch_mode(Mode::Manual).await?;

            // pad.change_speed(Speed::Two).await?;

            // pad.start_belt().await?;

            // std::thread::sleep(time::Duration::from_secs(10));

            pad.stop_belt().await?;

            pad.disconnect().await;
        } else {
            println!("Not found.")
        }
    }

    Ok(())
}
