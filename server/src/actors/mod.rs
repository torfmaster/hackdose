use std::cmp::{max, min};

use chrono::{DateTime, Duration, Utc};
use tokio::sync::mpsc::Receiver;

use crate::{
    actors::{
        ahoy_dtu::AhoyDtu,
        ez1m::EZ1M,
        marstek::{MarstekCharge, MarstekDischarge},
        opendtu::OpenDtu,
        tasmota::TasmotaSwitch,
    },
    ActorConfiguration, Configuration, RegulatingActorType, SwitchingActorType,
};

use self::hs100::HS100Switch;

mod ahoy_dtu;
mod ez1m;
mod hs100;
mod marstek;
mod opendtu;
mod tasmota;

struct ActorState {
    wait_until: Option<DateTime<Utc>>,
    time_until_effective_seconds: usize,
    special_state: ActorStateType,
}

impl ActorState {
    fn is_busy(&self) -> bool {
        self.wait_until > Some(Utc::now())
    }

    fn set_busy(&mut self) {
        self.wait_until = max(
            self.wait_until,
            Some(Utc::now() + Duration::seconds(self.time_until_effective_seconds as i64)),
        );
    }
}

enum ActorStateType {
    Switching(SwitchingActorState),
    Regulating(RegulatingActorState),
}
struct SwitchingActorState {
    switch: Box<dyn PowerSwitch + Send>,
    on: bool,
    power: usize,
}

struct RegulatingActorState {
    regulator: Box<dyn Regulator + Send>,
    max_power: usize,
}

impl ActorState {
    fn is_active(&self) -> bool {
        match &self.special_state {
            ActorStateType::Switching(switching_actor_state) => switching_actor_state.on,
            ActorStateType::Regulating(regulating_actor_state) => {
                regulating_actor_state.regulator.power() > 0
            }
        }
    }

    async fn turn_off(&mut self) {
        let dev = &mut self.special_state;
        match dev {
            ActorStateType::Switching(switching_actor_state) => {
                switching_actor_state.switch.off().await
            }
            ActorStateType::Regulating(regulating_actor_state) => {
                regulating_actor_state.regulator.change_power(0).await
            }
        }
    }

    async fn increase_effect_by(&mut self, power: isize) -> usize {
        if self.is_busy() {
            return 0;
        }
        match &mut self.special_state {
            ActorStateType::Switching(state) => {
                if state.power as isize > power {
                    return 0;
                }
                if state.on {
                    return 0;
                }

                state.switch.on().await;
                state.on = true;

                let power = state.power;
                self.set_busy();
                power
            }
            ActorStateType::Regulating(state) => {
                let remaining_potential =
                    state.max_power as isize - state.regulator.power() as isize;
                if remaining_potential <= 0 {
                    0
                } else {
                    state.regulator.change_power(power).await;
                    self.set_busy();
                    min(remaining_potential, power) as usize
                }
            }
        }
    }

    async fn reduce_effect_by(&mut self, power: usize) -> usize {
        if self.is_active() {
            match &mut self.special_state {
                ActorStateType::Switching(state) => {
                    state.switch.off().await;
                    state.on = false;
                    let power = state.power;
                    self.set_busy();
                    power
                }
                ActorStateType::Regulating(state) => {
                    let remaining_potential =
                        state.max_power as isize - state.regulator.power() as isize;
                    state.regulator.change_power(-(power as isize)).await;
                    self.set_busy();
                    min(remaining_potential, power as isize) as usize
                }
            }
        } else {
            0
        }
    }
}

#[async_trait::async_trait]
trait PowerSwitch {
    async fn on(&mut self);
    async fn off(&mut self);
}

#[async_trait::async_trait]
trait Regulator {
    async fn change_power(&mut self, power: isize);
    fn power(&self) -> usize;
}

impl ActorState {
    fn from_configuration(config: &ActorConfiguration) -> Self {
        match config {
            ActorConfiguration::Switching(switching_actor_configuration) => {
                match &switching_actor_configuration.actor {
                    SwitchingActorType::HS100(hs100_configuration) => {
                        let switch = Box::new(HS100Switch {
                            address: hs100_configuration.address.clone(),
                        });
                        ActorState {
                            time_until_effective_seconds: switching_actor_configuration
                                .time_until_effective_seconds,
                            wait_until: None,

                            special_state: ActorStateType::Switching(SwitchingActorState {
                                switch: switch,
                                on: false,
                                power: switching_actor_configuration.power,
                            }),
                        }
                    }

                    SwitchingActorType::Tasmota(tasmota_configuration) => {
                        let switch: Box<TasmotaSwitch> = Box::new(TasmotaSwitch {
                            url: tasmota_configuration.url.clone(),
                        });
                        ActorState {
                            time_until_effective_seconds: switching_actor_configuration
                                .time_until_effective_seconds,
                            wait_until: None,

                            special_state: ActorStateType::Switching(SwitchingActorState {
                                switch: switch,
                                on: false,
                                power: switching_actor_configuration.power,
                            }),
                        }
                    }
                }
            }
            ActorConfiguration::Regulating(regulating_actor_configuration) => {
                match &regulating_actor_configuration.actor {
                    RegulatingActorType::Ahoy(ahoy_configuration) => {
                        let regulator = Box::new(AhoyDtu {
                            url: ahoy_configuration.url.clone(),
                            upper_limit_watts: ahoy_configuration.upper_limit_watts,
                            inverter_no: ahoy_configuration.inverter_no,
                            current_watts: 0,
                        });
                        ActorState {
                            time_until_effective_seconds: regulating_actor_configuration
                                .time_until_effective_seconds,
                            wait_until: None,

                            special_state: ActorStateType::Regulating(RegulatingActorState {
                                regulator: regulator,
                                max_power: regulating_actor_configuration.max_power,
                            }),
                        }
                    }
                    RegulatingActorType::OpenDtu(open_dtu_configuration) => {
                        let regulator = Box::new(OpenDtu {
                            url: open_dtu_configuration.url.clone(),
                            upper_limit_watts: open_dtu_configuration.upper_limit_watts,
                            current_watts: 0,
                            serial: open_dtu_configuration.serial.clone(),
                            max_power: open_dtu_configuration.max_power,
                            password: open_dtu_configuration.password.clone(),
                        });
                        ActorState {
                            time_until_effective_seconds: regulating_actor_configuration
                                .time_until_effective_seconds,
                            wait_until: None,

                            special_state: ActorStateType::Regulating(RegulatingActorState {
                                regulator: regulator,
                                max_power: regulating_actor_configuration.max_power,
                            }),
                        }
                    }
                    RegulatingActorType::EZ1M(ez1m_configuration) => {
                        let regulator = Box::new(EZ1M {
                            url: ez1m_configuration.url.clone(),
                            upper_limit_watts: ez1m_configuration.upper_limit_watts,
                            current_watts: 0,
                        });
                        ActorState {
                            time_until_effective_seconds: regulating_actor_configuration
                                .time_until_effective_seconds,
                            wait_until: None,

                            special_state: ActorStateType::Regulating(RegulatingActorState {
                                regulator: regulator,
                                max_power: regulating_actor_configuration.max_power,
                            }),
                        }
                    }
                    RegulatingActorType::MarstekCharge(marstek_configuration) => {
                        let regulator = Box::new(MarstekCharge {
                            modbus_slave: marstek_configuration.modbus_slave.clone(),
                            upper_limit_watts: marstek_configuration.upper_limit_watts,
                            current_watts: 0,
                        });
                        ActorState {
                            time_until_effective_seconds: regulating_actor_configuration
                                .time_until_effective_seconds,
                            wait_until: None,

                            special_state: ActorStateType::Regulating(RegulatingActorState {
                                regulator: regulator,
                                max_power: regulating_actor_configuration.max_power,
                            }),
                        }
                    }
                    RegulatingActorType::MarstekDischarge(marstek_configuration) => {
                        let regulator = Box::new(MarstekDischarge {
                            modbus_slave: marstek_configuration.modbus_slave.clone(),
                            upper_limit_watts: marstek_configuration.upper_limit_watts,
                            current_watts: 0,
                        });
                        ActorState {
                            time_until_effective_seconds: regulating_actor_configuration
                                .time_until_effective_seconds,
                            wait_until: None,

                            special_state: ActorStateType::Regulating(RegulatingActorState {
                                regulator: regulator,
                                max_power: regulating_actor_configuration.max_power,
                            }),
                        }
                    }
                }
            }
        }
    }
}

enum SystemState {
    AllOff,
    Producing,
    Consuming,
}

fn compute_system_state(producers: &[ActorState], consumers: &[ActorState]) -> SystemState {
    if producers.iter().any(|p| p.is_active()) {
        SystemState::Producing
    } else if consumers.iter().any(|p| p.is_active()) {
        SystemState::Consuming
    } else {
        SystemState::AllOff
    }
}

pub(crate) async fn control_actors(
    rx: &mut Receiver<(i32, DateTime<Utc>)>,
    config: &Configuration,
) {
    let mut producers = config
        .actors
        .producers
        .iter()
        .map(|actor| ActorState::from_configuration(&actor))
        .collect::<Vec<_>>();

    let mut consumers = config
        .actors
        .consumers
        .iter()
        .map(|actor| ActorState::from_configuration(&actor))
        .collect::<Vec<_>>();

    let margin = (config.lower_limit + config.upper_limit) / 2;

    if producers.len() == 0 && consumers.len() == 0 {
        return;
    }

    for dev in producers.iter_mut() {
        dev.turn_off().await;
    }

    for dev in consumers.iter_mut() {
        dev.turn_off().await;
    }

    while let Some((received, timestamp)) = rx.recv().await {
        let system_state = compute_system_state(&producers, &consumers);
        let received = received as isize;

        // Skip events that are too old
        if timestamp > Utc::now() + Duration::seconds(5) {
            continue;
        }
        match system_state {
            SystemState::AllOff => {
                if received > config.upper_limit {
                    let mut difference_left = (received - margin) as isize;
                    for producer in producers.iter_mut() {
                        if difference_left <= 0 {
                            break;
                        }
                        let actual_effect =
                            producer.increase_effect_by(difference_left).await as isize;
                        difference_left -= actual_effect;
                    }
                }
                if received < config.lower_limit {
                    let mut difference_left = -(received - margin) as isize;
                    for consumer in consumers.iter_mut() {
                        if difference_left <= 0 {
                            break;
                        }
                        let actual_effect =
                            consumer.increase_effect_by(difference_left).await as isize;
                        difference_left -= actual_effect;
                    }
                }
            }
            SystemState::Producing => {
                if received < config.lower_limit {
                    let mut difference_left = -(received - margin) as isize;
                    for producer in producers.iter_mut().rev() {
                        if difference_left <= 0 {
                            break;
                        }
                        let actual_effect =
                            producer.reduce_effect_by(difference_left as usize).await as isize;
                        difference_left -= actual_effect;
                    }
                }

                if received > config.upper_limit {
                    let mut difference_left = (received - margin) as isize;
                    for producer in producers.iter_mut() {
                        if difference_left < 0 {
                            break;
                        }
                        let actual_effect =
                            producer.increase_effect_by(difference_left).await as isize;
                        difference_left -= actual_effect;
                    }
                }
            }
            SystemState::Consuming => {
                if received > config.upper_limit {
                    let mut difference_left = received - margin;
                    for consumer in consumers.iter_mut().rev() {
                        if difference_left <= 0 {
                            break;
                        }

                        let actual_effect =
                            consumer.reduce_effect_by(difference_left as usize).await as isize;
                        difference_left -= actual_effect;
                    }
                }

                if received < config.lower_limit {
                    let mut difference_left = -(received - margin) as isize;
                    for consumer in consumers.iter_mut() {
                        if difference_left <= 0 {
                            break;
                        }

                        let actual_effect =
                            consumer.increase_effect_by(difference_left).await as isize;
                        difference_left -= actual_effect;
                    }
                }
            }
        }
    }
}
