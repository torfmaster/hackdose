use chrono::{DateTime, Duration, Utc};
use tokio::sync::mpsc::Receiver;

use crate::{ActorConfiguration, ActorType, Configuration};

use self::{hs100::HS100Switch, tasmota::TasmotaSwitch};

mod hs100;
mod tasmota;

struct ActorState {
    disable_threshold: isize,
    enable_threshold: isize,
    duration_minutes: usize,
    last_set: Option<DateTime<Utc>>,
    switch: Box<dyn PowerSwitch + Send>,
    on: bool,
}

#[async_trait::async_trait]
trait PowerSwitch {
    async fn on(&mut self);
    async fn off(&mut self);
}

impl ActorState {
    fn from_configuration(config: &ActorConfiguration) -> Self {
        let switch: Box<dyn PowerSwitch + Send + 'static> = match &config.actor {
            ActorType::HS100(hs100) => Box::new(HS100Switch {
                address: hs100.address.clone(),
            }),
            ActorType::Tasmota(tasmota) => Box::new(TasmotaSwitch {
                url: tasmota.url.clone(),
            }),
        };
        Self {
            disable_threshold: config.disable_threshold,
            enable_threshold: config.enable_threshold,
            duration_minutes: config.duration_minutes,
            last_set: None,
            switch,
            on: false,
        }
    }
}

pub(crate) async fn control_actors(rx: &mut Receiver<i32>, config: &Configuration) {
    let mut devs = config
        .actors
        .iter()
        .map(|actor| ActorState::from_configuration(&actor))
        .collect::<Vec<_>>();

    if devs.len() == 0 {
        return;
    }

    while let Some(received) = rx.recv().await {
        let random_number = rand::random::<usize>() % devs.len();
        let dev = devs.get_mut(random_number).unwrap();
        let ActorState {
            disable_threshold,
            enable_threshold,
            duration_minutes,
            last_set,
            switch,
            on,
        } = dev;

        let should_be_on = if !*on {
            received < *enable_threshold as i32
        } else {
            !(received > *disable_threshold as i32)
        };
        if should_be_on != *on || last_set.is_none() {
            let now = chrono::Utc::now();
            if let Some(last_set_inner) = last_set {
                let diff = now - *last_set_inner;
                if diff > Duration::minutes(*duration_minutes as i64) {
                    *on = should_be_on;
                    *last_set = Some(now.clone());
                }
            } else {
                *on = should_be_on;
                *last_set = Some(chrono::Utc::now());
            }
            if *on {
                let _ = switch.on().await;
            } else {
                let _ = switch.off().await;
            }
        }
    }
}
