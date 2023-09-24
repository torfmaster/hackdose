/*

{"inverter":0,"cmd":11,"tx_request":81,"payload":[1500,0]}
POST http://ahoy-dtu/api/ctrl

*/

use std::time::Duration;

use reqwest::Url;
use serde::Serialize;

use super::PowerSwitch;

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

pub(crate) struct AhoyDtu {
    pub(crate) url: String,
    pub(crate) upper_limit_watts: usize,
    pub(crate) inverter_no: usize,
    pub(crate) current_watts: usize,
}

#[async_trait::async_trait]
impl PowerSwitch for AhoyDtu {
    async fn on(&mut self) {}

    async fn off(&mut self) {
        self.set_power(0).await;
    }

    async fn set_power(&mut self, power: isize) {
        let target = power + self.current_watts as isize;
        let target = if target > self.upper_limit_watts as isize {
            self.upper_limit_watts
        } else {
            if target > 0 {
                target as usize
            } else {
                0
            }
        };
        self.set_absolute(target).await;

        self.current_watts = target;
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
