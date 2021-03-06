// See the "macOS permissions note" in README.md before running this on macOS
// Big Sur or later.

use btleplug::api::{Central, Characteristic, Manager as _, Peripheral, ScanFilter};
use btleplug::platform::Manager;
extern crate derive_more;
use derive_more::{Add, Display, Error as DError, From, Into};
use futures::select;
use futures::StreamExt;
use std::error::Error;
use std::path::PathBuf;
use std::pin::Pin;
use std::time::Duration;
use structopt::StructOpt;
use tokio::time;
use warp::Filter;

use signal_hook::consts::signal::*;
use signal_hook_tokio::Signals;

mod controller;
use controller::*;

mod http;
use http::filters::*;

extern crate pretty_env_logger;
#[macro_use]
extern crate log;

async fn handle_signals(mut signals: Signals) {
    while let Some(signal) = signals.next().await {
        match signal {
            SIGHUP => {
                // Reload configuration
                // Reopen the log file
            }
            SIGTERM | SIGINT | SIGQUIT => break,
            _ => unreachable!(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init_timed();

    let signals = Signals::new(&[SIGHUP, SIGTERM, SIGINT, SIGQUIT])?;
    let handle = signals.handle();
    let signals_task = tokio::spawn(handle_signals(signals));

    let manager = Manager::new().await?;
    let adapter_list = manager.adapters().await?;
    if adapter_list.is_empty() {
        eprintln!("No Bluetooth adapters found");
    }

    let adapter = adapter_list[0].clone();

    info!("Starting scan on {}...", adapter.adapter_info().await?);
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
                        info!("ERROR {}", e);
                    }
                }
            }
            res
        };

        let walkingpad = x.await;

        if let Some(walkingpad) = walkingpad {
            let pad = Pad::new(&walkingpad).await?;

            let api = http::filters::walkingpad(pad.clone());
            tokio::spawn(async move {
                let routes = api.with(warp::log("walkingpad"));
                warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
            });
            pad.services().await;

            pad.subs().await?;

            let k = pad.gets().await?;
            tokio::spawn(async move {
                k.for_each(|data| async move {
                    let res = State::new(data.value);
                    info!("Received data [{:?}]: {:?}", data.uuid, res)
                })
                .await
            });

            info!("connected {}", walkingpad.is_connected().await?);
            // pad.start_belt().await?;
            pad.switch_mode(controller::enums::Mode::Manual).await?;

            let pad_clone = pad.clone();
            let j = tokio::spawn(async move {
                loop {
                    pad_clone.ask_stats().await;
                    tokio::time::sleep(Duration::from_millis(750)).await;
                }
            });

            // pad.switch_mode(Mode::Manual).await?;
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            pad.change_speed(25).await?;

            signals_task.await?;
            j.abort();
            // walkingpad.connect().await?;
            // pad.stop_belt().await?;

            pad.stop_belt().await?;
            pad.disconnect().await;
        } else {
            info!("Not found.")
        }
    }

    Ok(())
}
