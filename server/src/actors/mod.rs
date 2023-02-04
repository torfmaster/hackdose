use chrono::{DateTime, Duration, Utc};
use tokio::sync::mpsc::Receiver;

use crate::{ActorConfiguration, Configuration};

use self::{hs100::HS100Switch, tasmota::TasmotaSwitch};

mod hs100;
mod tasmota;

struct ActorState {
    disable_threshold: isize,
    enable_threshold: isize,
    duration_minutes: usize,
    last_set: Option<DateTime<Utc>>,
    switch: Box<dyn PowerSwitch + Send>,
}

#[async_trait::async_trait]
trait PowerSwitch {
    async fn on(&mut self);
    async fn off(&mut self);
}

impl ActorState {
    fn from_configuration(config: &ActorConfiguration) -> Self {
        match config {
            ActorConfiguration::HS100(hs100) => ActorState {
                disable_threshold: hs100.disable_threshold,
                enable_threshold: hs100.enable_threshold,
                duration_minutes: hs100.duration_minutes,
                last_set: None,
                switch: Box::new(HS100Switch {
                    address: hs100.address.clone(),
                }),
            },
            ActorConfiguration::Tasmota(tasmota) => ActorState {
                disable_threshold: tasmota.disable_threshold,
                enable_threshold: tasmota.enable_threshold,
                duration_minutes: tasmota.duration_minutes,
                last_set: None,
                switch: Box::new(TasmotaSwitch {
                    url: tasmota.url.clone(),
                }),
            },
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

    let mut on = false;

    while let Some(received) = rx.recv().await {
        let random_number = rand::random::<usize>() % devs.len();
        let dev = devs.get_mut(random_number).unwrap();
        let ActorState {
            disable_threshold,
            enable_threshold,
            duration_minutes,
            last_set,
            switch,
        } = dev;

        let should_be_on = if !on {
            received < *enable_threshold as i32
        } else {
            !(received > *disable_threshold as i32)
        };
        if should_be_on != on {
            let now = chrono::Utc::now();
            if let Some(last_set_inner) = last_set {
                let diff = now - *last_set_inner;
                if diff > Duration::minutes(*duration_minutes as i64) {
                    on = should_be_on;
                    *last_set = Some(now.clone());
                }
            } else {
                on = should_be_on;
                *last_set = Some(chrono::Utc::now());
            }
        }

        if on {
            let _ = switch.on().await;
        } else {
            let _ = switch.off().await;
        }
    }
}
