use chrono::{DateTime, Duration, Utc};
use tokio::sync::mpsc::Receiver;

use crate::{ActorConfiguration, ActorMode, ActorType, Configuration};

use self::{ahoy_dtu::AhoyDtu, hs100::HS100Switch, tasmota::TasmotaSwitch};

mod ahoy_dtu;
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
    last_updated: Option<DateTime<Utc>>,
}

#[async_trait::async_trait]
trait PowerSwitch {
    async fn on(&mut self);
    async fn off(&mut self);
    async fn set_power(&mut self, power: isize);
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
            ActorType::Ahoy(ahoy) => Box::new(AhoyDtu {
                url: ahoy.url.clone(),
                upper_limit_watts: ahoy.upper_limit_watts,
                inverter_no: ahoy.inverter_no,
                current_watts: 0,
            }),
        };
        Self {
            disable_threshold: config.disable_threshold,
            enable_threshold: config.enable_threshold,
            duration_minutes: config.duration_minutes,
            last_set: None,
            last_updated: None,
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

    for dev in devs.iter_mut() {
        dev.switch.off().await;
        dev.switch.set_power(0).await;
    }

    while let Some(received) = rx.recv().await {
        let dev = get_actor(&mut devs);
        let ActorState {
            disable_threshold,
            enable_threshold,
            duration_minutes,
            last_set,
            switch,
            actor_mode,
            on,
            last_updated,
        } = dev;

        let should_be_on = compute_actor_state(
            *on,
            received,
            *enable_threshold,
            *disable_threshold,
            actor_mode.clone(),
        );
        let now = chrono::Utc::now();

        if should_be_on != *on {
            if let Some(last_set_inner) = last_set {
                let diff = now - *last_set_inner;
                if diff > Duration::minutes(*duration_minutes as i64) {
                    *on = should_be_on;
                    *last_set = Some(now.clone());
                    if *on {
                        let _ = switch.on().await;
                    } else {
                        let _ = switch.off().await;
                    }
                }
            } else {
                *on = should_be_on;
                *last_set = Some(chrono::Utc::now());
                if *on {
                    let _ = switch.on().await;
                } else {
                    let _ = switch.off().await;
                }
            }
        }

        if let Some(last_updated_inner) = last_updated {
            let diff = now - *last_updated_inner;
            if diff > Duration::minutes(*duration_minutes as i64) {
                *last_updated = Some(now);
                dbg!("refresh");
                let _ = switch.set_power(received as isize).await;
            } else {
                dbg!("skip");
            }
        } else {
            *last_updated = Some(now);
            dbg!("first");
            let _ = switch.set_power(received as isize).await;
        }
    }
}

fn get_actor(devs: &mut Vec<ActorState>) -> &mut ActorState {
    devs.sort_by(|x, y| (&x.actor_mode).cmp(&y.actor_mode));
    let index = devs.iter().position(|d| d.actor_mode == ActorMode::Charge);
    if let Some(index) = index {
        let (dischargers, chargers) = devs.split_at_mut(index);

        if dischargers.len() > 0 && dischargers.iter().any(|d| d.on) {
            return get_random_element(dischargers).unwrap();
        }
        if chargers.len() > 0 && chargers.iter().any(|d| d.on) {
            return get_random_element(chargers).unwrap();
        }

        if dischargers.len() > 0 && rand::random::<bool>() {
            return get_random_element(dischargers).unwrap();
        }
        return get_random_element(chargers).unwrap();
    } else {
        return get_random_element(devs).unwrap();
    }
}

fn get_random_element<T>(arr: &mut [T]) -> Option<&mut T> {
    if arr.len() == 0 {
        return None;
    }
    let random_number = rand::random::<usize>() % arr.len();
    let dev = arr.get_mut(random_number);
    dev
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

    struct DummyActor;

    #[async_trait::async_trait]
    impl PowerSwitch for DummyActor {
        async fn on(&mut self) {}

        async fn off(&mut self) {}
        async fn set_power(&mut self, _: isize) {}
    }

    #[test]
    pub fn returns_a_charger_if_a_charger_is_turned_on() {
        let mut devs = vec![
            actor(ActorMode::Charge, true),
            actor(ActorMode::Discharge, false),
        ];
        let result = get_actor(&mut devs);

        assert_eq!(result.on, true);
    }

    #[test]
    pub fn returns_a_discharger_if_a_discharger_is_turned_on() {
        let mut devs = vec![
            actor(ActorMode::Charge, false),
            actor(ActorMode::Discharge, true),
        ];
        let result = get_actor(&mut devs);

        assert_eq!(result.actor_mode, ActorMode::Discharge);
        assert_eq!(result.on, true);
    }
    #[test]
    pub fn returns_any_turned_off_element_if_no_actor_is_turned_on() {
        let mut devs = vec![
            actor(ActorMode::Charge, false),
            actor(ActorMode::Discharge, false),
        ];
        let result = get_actor(&mut devs);

        assert_eq!(result.on, false);
    }

    fn actor(actor_mode: ActorMode, on: bool) -> ActorState {
        ActorState {
            disable_threshold: 100,
            enable_threshold: -100,
            duration_minutes: 60,
            last_set: None,
            switch: Box::new(DummyActor),
            on,
            actor_mode,
            last_updated: None,
        }
    }
}
