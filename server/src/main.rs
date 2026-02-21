use business::handle_power_events;
use chrono::{DateTime, Utc};
use clap::Parser;
use data::EnergyData;
use hackdose_sml_parser::application::domain::AnyValue;
use hackdose_sml_parser::application::obis::Obis;
use hackdose_sml_parser::message_stream::sml_message_stream;
use serde::{Deserialize, Serialize};
use smart_meter::{enable_ir_sensor_power_supply, uart_ir_sensor_data_stream};
use std::sync::Arc;
use std::time::Duration;
use std::{collections::HashMap, path::PathBuf};
use tokio::fs::File;
use tokio::io::BufReader;
use tokio::time::sleep;

use actors::control_actors;
use rest::serve_rest_endpoint;
use tokio::io::AsyncReadExt;

use crate::actors::rd6006::RD6006Config;
use crate::config::ModbusSlave;
use crate::smart_meter::generic_modbus::spawn_modbus_producer;

mod actors;
mod business;
mod config;
mod data;
mod rest;
mod smart_meter;

#[derive(Serialize, Deserialize, Clone)]
struct SwitchingActorConfiguration {
    actor: SwitchingActorType,
    time_until_effective_seconds: usize,
    power: usize,
}

#[derive(Serialize, Deserialize, Clone)]
struct RegulatingActorConfiguration {
    actor: RegulatingActorType,
    time_until_effective_seconds: usize,
    max_power: usize,
}

#[derive(Serialize, Deserialize, Clone)]
enum ActorConfiguration {
    Switching(SwitchingActorConfiguration),
    Regulating(RegulatingActorConfiguration),
}

#[derive(Serialize, Deserialize, Clone)]
enum SwitchingActorType {
    HS100(HS100Configuration),
    Tasmota(TasmotaConfiguration),
}

#[derive(Serialize, Deserialize, Clone)]
enum RegulatingActorType {
    Ahoy(AhoyConfiguration),
    OpenDtu(OpenDtuConfiguration),
    MarstekCharge(MarstekConfiguration),
    MarstekDischarge(MarstekConfiguration),
    EZ1M(EZ1MConfiguration),
    RD6006(RD6006Config),
}
#[derive(Serialize, Deserialize, Clone)]
struct HS100Configuration {
    address: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct TasmotaConfiguration {
    url: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct AhoyConfiguration {
    upper_limit_watts: usize,
    url: String,
    inverter_no: usize,
}

#[derive(Serialize, Deserialize, Clone)]
struct OpenDtuConfiguration {
    serial: String,
    max_power: usize,
    password: String,
    url: String,
    upper_limit_watts: usize,
}

#[derive(Serialize, Deserialize, Clone)]
struct EZ1MConfiguration {
    url: String,
    upper_limit_watts: usize,
}

#[derive(Serialize, Deserialize, Clone)]

pub(crate) struct MarstekConfiguration {
    pub(crate) modbus_slave: ModbusSlave,
    pub(crate) upper_limit_watts: usize,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct Configuration {
    actors: Actors,
    log_location: PathBuf,
    power_measurement: PowerMeasurement,
    lower_limit: isize,
    upper_limit: isize,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct SmlSmartMeterData {
    gpio_location: Option<String>,
    ttys_location: String,
    gpio_power_pin: u32,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct GenericModbusCounterSlaveData {
    modbus_slave: ModbusSlave,
    // holds the current effective power
    register: u16,
    poll_intervall_millis: usize,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) enum PowerMeasurement {
    SmlSmartMeter(SmlSmartMeterData),
    GenericModbusSlave(GenericModbusCounterSlaveData),
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct Actors {
    consumers: Vec<ActorConfiguration>,
    producers: Vec<ActorConfiguration>,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, value_name = "FILE")]
    config: PathBuf,
}

impl Args {
    async fn get_config_file(&self) -> Configuration {
        let config = File::open(&self.config).await.unwrap();
        let mut config_file = String::new();
        BufReader::new(config)
            .read_to_string(&mut config_file)
            .await
            .unwrap();
        serde_yaml::from_str::<Configuration>(&config_file).unwrap()
    }
}

#[tokio::main(worker_threads = 2)]
async fn main() {
    let args = Args::parse();
    let config = args.get_config_file().await;
    let energy_data = EnergyData::try_from_file(&config).await;
    let (mut tx, mut rx) = tokio::sync::mpsc::channel::<(i32, DateTime<Utc>)>(100);
    let mutex = Arc::new(tokio::sync::Mutex::new(HashMap::<Obis, AnyValue>::new()));

    match &config.power_measurement {
        PowerMeasurement::SmlSmartMeter(sml_smart_meter_data) => {
            enable_ir_sensor_power_supply(&sml_smart_meter_data);

            let stream = uart_ir_sensor_data_stream(&sml_smart_meter_data);
            let power_events = sml_message_stream(stream);

            let power_event_mutex = mutex.clone();
            let energy_data_power = energy_data.clone();

            tokio::task::spawn(async move {
                handle_power_events(&mut tx, power_event_mutex, power_events, energy_data_power)
                    .await
            });
        }
        PowerMeasurement::GenericModbusSlave(config) => {
            let energy_data_power = energy_data.clone();

            spawn_modbus_producer(config, &mut tx, energy_data_power);
        }
    }

    let control_actors_config = config.clone();
    tokio::task::spawn(async move { control_actors(&mut rx, &control_actors_config).await });

    let rest_event_mutex = mutex.clone();
    let rest_config = config.clone();
    let rest_energy_data = energy_data.clone();
    let rotate_energy_data = energy_data.clone();

    tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(86400)).await;
            rotate_energy_data.rotate_log().await;
        }
    });

    serve_rest_endpoint(rest_event_mutex, rest_energy_data, &rest_config).await;
}

#[cfg(test)]
mod test {
    use super::*;
    #[tokio::test]
    pub async fn deserializes_sample_config() {
        let config = File::open("config-sample.yaml").await.unwrap();
        let mut config_file = String::new();
        BufReader::new(config)
            .read_to_string(&mut config_file)
            .await
            .unwrap();
        serde_yaml::from_str::<Configuration>(&config_file).unwrap();
    }
}
