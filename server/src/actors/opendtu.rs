use std::{
    cmp::{max, min},
    collections::HashMap,
    time::Duration,
};

use reqwest::Url;
use serde::{Deserialize, Serialize};

use crate::actors::Regulator;

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct OpenDtuConfiguration {
    pub(crate) serial: String,
    pub(crate) max_power: usize,
    pub(crate) password: String,
    pub(crate) url: String,
    pub(crate) upper_limit_watts: usize,
}

pub(crate) struct OpenDtu {
    pub(crate) serial: String,
    pub(crate) max_power: usize,
    pub(crate) password: String,
    pub(crate) url: String,

    pub(crate) current_watts: usize,
    pub(crate) upper_limit_watts: usize,
}

#[async_trait::async_trait]
impl Regulator for OpenDtu {
    async fn change_power(&mut self, power: isize) {
        let target = (power + self.current_watts as isize) as isize;
        let target = max(0, target);
        let target = min(target, self.upper_limit_watts as isize);

        self.current_watts = target as usize;
        self.set_absolute(target as usize).await;
    }

    fn power(&self) -> usize {
        self.current_watts
    }
}

impl OpenDtu {
    async fn set_absolute(&mut self, power: usize) {
        let serial = self.serial.clone();
        let max_power = self.max_power;
        let limit_value_watts = power;
        let password = self.password.clone();
        let url = self.url.clone();

        let rel_limit = limit_value_watts * 100 / max_power;
        let payload = serde_json::to_string(&LimitPayload {
            serial,
            limit_type: LimitType::Relative,
            limit_value: rel_limit,
        })
        .unwrap();

        let mut form = HashMap::new();
        form.insert("data", payload);

        let mut url = Url::parse(&url).unwrap();
        url.set_path("/api/limit/config");

        let client_builder = reqwest::ClientBuilder::new();
        let client_builder = client_builder.connect_timeout(Duration::from_secs(5));
        let client = client_builder.build().unwrap();

        let _ = client
            .post(url)
            .basic_auth("admin", Some(password))
            .form(&form)
            .send()
            .await;
    }
}

pub(crate) enum LimitType {
    Relative = 1,
}

impl Serialize for LimitType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        return serializer.serialize_u64(match self {
            LimitType::Relative => 1,
        });
    }
}

#[derive(Serialize)]
pub(crate) struct LimitPayload {
    serial: String,
    limit_type: LimitType,
    limit_value: usize,
}
