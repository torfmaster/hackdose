use std::{
    cmp::{max, min},
    time::Duration,
};

use reqwest::Url;
use serde::{Deserialize, Serialize};
use tokio::task;

use crate::actors::Regulator;

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct EZ1MConfiguration {
    pub(crate) url: String,
    pub(crate) upper_limit_watts: usize,
}

pub(crate) struct EZ1M {
    pub(crate) url: String,

    pub(crate) current_watts: usize,
    pub(crate) upper_limit_watts: usize,
}

#[async_trait::async_trait]
impl Regulator for EZ1M {
    async fn change_power(&mut self, power: isize) {
        let target = (power + self.current_watts as isize) as isize;
        let target = max(0, target);
        let target = min(target, self.upper_limit_watts as isize);

        self.current_watts = target as usize;
        if target < 30 {
            self.turn_off().await;
        } else {
            self.turn_on().await;
            self.set_absolute(target as usize).await;
        }
    }

    fn power(&self) -> usize {
        self.current_watts
    }
}

impl EZ1M {
    async fn set_absolute(&mut self, power: usize) {
        let url = self.url.clone();
        let power = power * 2;

        let mut url = Url::parse(&url).unwrap();
        url.set_path("/setMaxPower");

        let client_builder = reqwest::ClientBuilder::new();
        let client_builder = client_builder.connect_timeout(Duration::from_secs(5));
        let client = client_builder.build().unwrap();

        task::spawn(client.get(url).query(&[("p", power)]).send());
    }

    async fn turn_off(&mut self) {
        let url = self.url.clone();

        let mut url = Url::parse(&url).unwrap();
        url.set_path("/setOnOff");

        let client_builder = reqwest::ClientBuilder::new();
        let client_builder = client_builder.connect_timeout(Duration::from_secs(5));
        let client = client_builder.build().unwrap();

        task::spawn(client.get(url).query(&[("status", 1)]).send());
    }

    async fn turn_on(&mut self) {
        let url = self.url.clone();

        let mut url = Url::parse(&url).unwrap();
        url.set_path("/setOnOff");

        let client_builder = reqwest::ClientBuilder::new();
        let client_builder = client_builder.connect_timeout(Duration::from_secs(5));
        let client = client_builder.build().unwrap();

        task::spawn(client.get(url).query(&[("status", 0)]).send());
    }
}

mod test {
    use std::time::Duration;

    use reqwest::Url;

    // #[tokio::test]
    pub async fn lae() {
        let power = 33;
        let url = "http://192.168.178.54:8050/".to_string();

        let mut url = Url::parse(&url).unwrap();
        url.set_path("/setMaxPower");

        let client_builder = reqwest::ClientBuilder::new();
        let client_builder = client_builder.connect_timeout(Duration::from_secs(5));
        let client = client_builder.build().unwrap();

        let n = client.get(url).query(&[("p", 31)]).send().await;
        dbg!(n);
    }
}
