use clap::Parser;
use hackdose_sml_parser::application::obis::Obis;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::io::BufReader;

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct Configuration {
    pub(crate) ttys_location: String,
    pub(crate) mqtt_broker: MqttBroker,
    pub(crate) publications: Vec<Publication>,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct MqttBroker {
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) username: String,
    pub(crate) password: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct Publication {
    pub(crate) topic: String,
    pub(crate) obis: Obis,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub(crate) struct Args {
    #[arg(short, long, value_name = "FILE")]
    config: PathBuf,
}

impl Args {
    pub(crate) async fn get_config_file(&self) -> Configuration {
        let config = File::open(&self.config).await.unwrap();
        let mut config_file = String::new();
        BufReader::new(config)
            .read_to_string(&mut config_file)
            .await
            .unwrap();
        serde_yaml::from_str::<Configuration>(&config_file).unwrap()
    }
}
