// See the "macOS permissions note" in README.md before running this on macOS
// Big Sur or later.

use btleplug::api::{Central, Characteristic, Manager as _, Peripheral, ScanFilter};
use btleplug::platform::Manager;
extern crate derive_more;
use derive_more::{Add, Display, Error as DError, From, Into};
use std::error::Error;
use std::time::Duration;
use tokio::time;

use std::collections::BTreeSet;

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

#[derive(Display, Debug, DError)]
struct MyError {
    details: String,
}

#[repr(u8)]
enum Mode {
    Standby = 2,
    Manual = 1,
    Automat = 0,
}

impl Mode {
    fn to_u8(&self) -> u8 {
        match self {
            Mode::Automat => 0,
            Mode::Manual => 1,
            Mode::Standby => 2,
        }
    }
}

enum Speed {
    Zero = 0,
    One = 1,
    Two = 2,
    Three = 3,
    Four = 4,
    Five = 5,
    Six = 6,
}

impl Speed {
    fn to_u8(&self) -> u8 {
        match self {
            Speed::Zero => 0,
            Speed::One => 1,
            Speed::Two => 2,
            Speed::Three => 3,
            Speed::Four => 4,
            Speed::Five => 5,
            Speed::Six => 6,
        }
    }
}

struct Pad<T: Peripheral> {
    char_fe01: Characteristic,
    char_fe02: Characteristic,
    peripheral: T,
}

const FE01: &str = "0000fe01";
const FEO2: &str = "0000fe02";

impl<T: Peripheral> Pad<T> {
    async fn services(&self) {
        println!("Discover peripheral services...");
        for service in self.peripheral.services() {
            println!(
                "Service UUID {}, primary: {}",
                service.uuid, service.primary
            );
            for characteristic in service.characteristics {
                println!("  {:?}", characteristic);
                // walkingpad.write(characteristic,  ,WriteType::WithoutResponse)
            }
        }
    }

    async fn disconnect(&self) {
        self.peripheral
            .disconnect()
            .await
            .expect("Error disconnecting from BLE peripheral");
    }

    async fn get_char(
        match_str: &str,
        characteristics: BTreeSet<Characteristic>,
    ) -> Result<Characteristic, Box<dyn Error>> {
        for char in characteristics.iter() {
            if char.uuid.to_string().contains(match_str) {
                return Ok(char.clone());
            }
        }

        Err(Box::new(MyError {
            details: "Not found".to_string(),
        }))
    }

    async fn new(peripheral: &T) -> Result<Pad<T>, Box<dyn Error>> {
        let is_connected = peripheral.is_connected().await?;

        if !is_connected {
            let connected = peripheral.connect().await;
            match connected {
                Ok(_) => println!("Connected!"),
                Err(e) => {
                    return Err(Box::new(MyError {
                        details: e.to_string(),
                    }))
                }
            }
        }

        if peripheral.characteristics().len() == 0 {
            peripheral.discover_services().await?;
        }

        let char01 = Pad::<T>::get_char(FE01, peripheral.characteristics()).await?;
        let char02 = Pad::<T>::get_char(FEO2, peripheral.characteristics()).await?;

        Ok(Pad {
            char_fe01: char01,
            char_fe02: char02,
            peripheral: peripheral.clone(),
        })
    }

    fn crc(cmd: &[u8]) -> u8 {
        (((cmd[1] + cmd[2] + cmd[3]) as u32).rem_euclid(256)) as u8
    }

    // fn set_pref_arr(key: u8, arr: &[u8]) -> [u8] {
    //     arr.it
    //     let ar: [u32] = vec![8].try_into().unwrap_or([0]);

    //     [[&[247, 166, key], arr].concat(), [172, 253]].concat()
    // }

    async fn change_speed(&self, speed: Speed) -> Result<(), btleplug::Error> {
        let cmd = [247, 162, 1, speed.to_u8(), 255, 253];
        let cmd = [247, 162, 1, speed.to_u8(), Pad::<T>::crc(&cmd), 253];

        self.peripheral
            .write(
                &self.char_fe02,
                &cmd,
                btleplug::api::WriteType::WithoutResponse,
            )
            .await
    }

    async fn stop_belt(&self) -> Result<(), btleplug::Error> {
        self.change_speed(Speed::Zero).await
    }

    async fn start_belt(&self) -> Result<(), btleplug::Error> {
        let cmd = [247, 162, 4, 1, 255, 253];
        let cmd = [247, 162, 4, 1, Pad::<T>::crc(&cmd), 253];
        self.peripheral
            .write(
                &self.char_fe02,
                &cmd,
                btleplug::api::WriteType::WithoutResponse,
            )
            .await
    }

    async fn switch_mode(&self, mode: Mode) -> Result<(), btleplug::Error> {
        let m = mode.to_u8();
        let cmd: [u8; 6] = [247, 162, 2, m, 255, 253];
        let cmd: [u8; 6] = [247, 162, 2, m, Pad::<T>::crc(&cmd), 253];
        self.peripheral
            .write(
                &self.char_fe02,
                &cmd,
                btleplug::api::WriteType::WithoutResponse,
            )
            .await
    }
}
