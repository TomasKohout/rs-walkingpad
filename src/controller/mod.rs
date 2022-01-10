mod enums;
use enums::*;

mod error;
use error::*;

use btleplug::api::{Characteristic, Peripheral, ValueNotification};
use std::collections::BTreeSet;
use std::error::Error;

use futures::stream::Stream;

use tokio::sync::watch::Sender;

use std::sync::Arc;
use tokio::sync::RwLock;

use std::pin::Pin;

const FE01: &str = "0000fe01";
const FEO2: &str = "0000fe02";
#[derive(Debug)]
pub struct Pad<T: Peripheral> {
    char_fe01: Characteristic,
    char_fe02: Characteristic,
    peripheral: T,
    subscribers: Arc<RwLock<Vec<Sender<Message>>>>,
}
impl<T: Peripheral> Pad<T> {
    pub async fn new(peripheral: &T) -> Result<Pad<T>, Box<dyn Error>> {
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
            subscribers: Arc::new(RwLock::new(vec![])),
        })
    }

    pub async fn stop_belt(&self) -> Result<(), btleplug::Error> {
        self.change_speed(Speed::Zero).await
    }

    pub async fn start_belt(&self) -> Result<(), btleplug::Error> {
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

    pub async fn switch_mode(&self, mode: Mode) -> Result<(), btleplug::Error> {
        let m = mode as u8;
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

    pub async fn change_speed(&self, speed: u8) -> Result<(), btleplug::Error> {
        let cmd = [247, 162, 1, speed, 255, 253];
        let cmd = [247, 162, 1, speed, Pad::<T>::crc(&cmd), 253];

        self.peripheral
            .write(
                &self.char_fe02,
                &cmd,
                btleplug::api::WriteType::WithoutResponse,
            )
            .await
    }

    pub async fn disconnect(&self) {
        self.peripheral
            .disconnect()
            .await
            .expect("Error disconnecting from BLE peripheral");
    }

    pub async fn services(&self) {
        println!("Discover peripheral services...");
        for service in self.peripheral.services() {
            println!(
                "Service UUID {}, primary: {}",
                service.uuid, service.primary
            );
            for characteristic in service.characteristics {
                println!("  {:?}", characteristic);
            }
        }
    }

    pub async fn register(self, chan: Sender<Message>) {
        self.subscribers.write().await.push(chan);
    }

    pub async fn subs(&self) -> Result<(), btleplug::Error> {
        self.peripheral.subscribe(&self.char_fe01).await
    }

    pub async fn gets(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = ValueNotification> + Send>>, btleplug::Error> {
        self.peripheral.notifications().await
    }

    pub async fn ask_stats(&self) -> Result<(), btleplug::Error> {
        let cmd: [u8; 6] = [247, 162, 0, 0, 162, 253];
        let cmd = [247, 162, 0, 0, Pad::<T>::crc(&cmd), 253];
        self.peripheral
            .write(
                &self.char_fe02,
                &cmd,
                btleplug::api::WriteType::WithoutResponse,
            )
            .await
    }

    pub async fn ask_profile(&self) -> Result<(), btleplug::Error> {
        let cmd = [247, 165, 96, 74, 77, 147, 113, 41, 201, 253];
        let cmd = [247, 165, 96, 74, 77, 147, 113, 41, Pad::<T>::crc(&cmd), 253];
        self.peripheral
            .write(
                &self.char_fe02,
                &cmd,
                btleplug::api::WriteType::WithoutResponse,
            )
            .await
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

    fn crc(cmd: &[u8]) -> u8 {
        let k = cmd[1..cmd.len() - 2]
            .to_vec()
            .iter()
            .fold(0, |acc: u32, val| acc + *val as u32);
        (k as u32).rem_euclid(256) as u8
    }

    // fn set_pref_arr(key: u8, arr: &[u8]) -> [u8] {
    //     arr.it
    //     let ar: [u32] = vec![8].try_into().unwrap_or([0]);

    //     [[&[247, 166, key], arr].concat(), [172, 253]].concat()
    // }
}

#[derive(Debug)]
pub struct State {
    pub belt_state: BeltState,
    pub speed: usize,
    pub mode: Mode,
    pub time: usize,
    pub distance: usize,
    pub steps: usize,
    pub last_speed: usize,
}

impl State {
    fn convert(data: &[u8]) -> usize {
        let mut value: usize = 0;
        for i in 0..data.len() {
            value += ((data[i] as usize) << (8 * (data.len() - 1 - i))) as usize;
        }

        value
    }
    fn check_data(data: &Vec<u8>) -> bool {
        if data.len() < 2 {
            false
        } else if data[0] == 248 && data[1] == 162 {
            true
        } else {
            false
        }
    }
    pub fn new(data: Vec<u8>) -> Option<Self> {
        if State::check_data(&data) {
            let belt_state: BeltState = data[2].into();
            let speed: usize = data[3].into();
            let mode: Mode = data[4].into();
            let time: usize = State::convert(&data[5..8]);
            let distance: usize = State::convert(&data[8..11]);
            let steps: usize = State::convert(&data[11..14]);
            let last_speed: usize = data[14].into();

            Some(Self {
                belt_state,
                speed,
                mode,
                time,
                distance,
                steps,
                last_speed,
            })
        } else {
            None
        }
    }
}
