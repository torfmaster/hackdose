use std::{
    cmp::{max, min},
    time::Duration,
};

use reqwest::Url;
use serde::{Deserialize, Serialize};

use crate::actors::Regulator;

#[derive(Serialize)]
struct Payload {
    inverter: usize,
    cmd: usize,
    tx_request: usize,
    payload: Vec<usize>,
}

impl Payload {
    fn for_watts(inverter: usize, watts: usize) -> Self {
        Payload {
            inverter,
            cmd: 11,
            tx_request: 81,
            payload: vec![watts, 0],
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct AhoyConfiguration {
    pub(crate) upper_limit_watts: usize,
    pub(crate) url: String,
    pub(crate) inverter_no: usize,
}
pub(crate) struct AhoyDtu {
    pub(crate) url: String,
    pub(crate) upper_limit_watts: usize,
    pub(crate) inverter_no: usize,
    pub(crate) current_watts: usize,
}

#[async_trait::async_trait]
impl Regulator for AhoyDtu {
    async fn change_power(&mut self, power: isize) {
        let target = (power + self.current_watts as isize) as isize;
        let target = max(0, target);
        let target = min(target, self.upper_limit_watts as isize);
        self.set_absolute(target as usize).await;

        self.current_watts = target as usize;
    }

    fn power(&self) -> usize {
        self.current_watts
    }
}

impl AhoyDtu {
    async fn set_absolute(&mut self, power: usize) {
        let payload = Payload::for_watts(self.inverter_no, power);
        let serialized = serde_json::ser::to_string(&payload).unwrap();
        let mut url = Url::parse(&self.url).unwrap();
        url.set_path("/api/ctrl");

        let client_builder = reqwest::ClientBuilder::new();
        let client_builder = client_builder.connect_timeout(Duration::from_secs(5));
        let client = client_builder.build().unwrap();
        let _ = client.post(url).body(serialized).send().await;
    }
}
