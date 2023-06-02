use chrono::{DateTime, Duration, Utc};
use tokio::sync::mpsc::Receiver;

use crate::{ActorConfiguration, ActorMode, ActorType, Configuration};

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
    actor_mode: ActorMode,
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
            actor_mode: config.actor_mode.clone(),
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
            actor_mode,
            on,
        } = dev;

        let should_be_on = compute_actor_state(
            *on,
            received,
            *enable_threshold,
            *disable_threshold,
            actor_mode.clone(),
        ) || last_set.is_none();
        if should_be_on != *on {
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

fn compute_actor_state(
    on: bool,
    received: i32,
    enable_threshold: isize,
    disable_threshold: isize,
    actor_mode: ActorMode,
) -> bool {
    match actor_mode {
        ActorMode::Charge => {
            if !on {
                received < enable_threshold as i32
            } else {
                !(received > disable_threshold as i32)
            }
        }
        ActorMode::Discharge => {
            if !on {
                received > enable_threshold as i32
            } else {
                !(received < disable_threshold as i32)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::ActorMode;

    use super::*;

    #[test]
    pub fn charges_if_below_threshold() {
        let should_be_on = compute_actor_state(false, -100, -50, 100, ActorMode::Charge);
        assert_eq!(should_be_on, true);
    }

    #[test]
    pub fn does_nothing_if_charging_and_if_below_threshold() {
        let should_be_on = compute_actor_state(true, -100, -50, 100, ActorMode::Charge);
        assert_eq!(should_be_on, true);
    }

    #[test]
    pub fn stays_on_if_charging_and_if_between_thresholds() {
        let should_be_on = compute_actor_state(true, -30, -50, 100, ActorMode::Charge);
        assert_eq!(should_be_on, true);
    }

    #[test]
    pub fn stops_charging_if_above_threshold() {
        let should_be_on = compute_actor_state(true, 200, -50, 100, ActorMode::Charge);
        assert_eq!(should_be_on, false);
    }

    #[test]
    pub fn stays_not_charging_if_above_threshold() {
        let should_be_on = compute_actor_state(false, 200, -50, 100, ActorMode::Charge);
        assert_eq!(should_be_on, false);
    }

    #[test]
    pub fn discharges_if_above_threshold() {
        let should_be_on = compute_actor_state(false, 200, 100, 0, ActorMode::Discharge);
        assert_eq!(should_be_on, true);
    }

    #[test]
    pub fn keeps_on_discharging_if_above_threshold() {
        let should_be_on = compute_actor_state(true, 200, 100, 0, ActorMode::Discharge);
        assert_eq!(should_be_on, true);
    }

    #[test]
    pub fn stays_on_if_discharging_and_if_between_thresholds() {
        let should_be_on = compute_actor_state(true, 50, 100, 0, ActorMode::Discharge);
        assert_eq!(should_be_on, true);
    }

    #[test]
    pub fn stops_discharging_if_below_threshold() {
        let should_be_on = compute_actor_state(false, -100, 100, 0, ActorMode::Discharge);
        assert_eq!(should_be_on, false);
    }

    #[test]
    pub fn stays_not_discharging_if_below_threshold() {
        let should_be_on = compute_actor_state(false, -100, 100, 0, ActorMode::Discharge);
        assert_eq!(should_be_on, false);
    }
}
