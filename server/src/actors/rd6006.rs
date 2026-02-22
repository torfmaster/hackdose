use std::cmp::{max, min};

use serde::{Deserialize, Serialize};
use tokio::task;

use crate::{
    actors::{
        ActorState, ActorStateType, RegulatingActorConfiguration, RegulatingActorState, Regulator,
    },
    config::ModbusSlave,
};
use tokio_modbus::{client::Context, prelude::*};

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct RD6006Config {
    pub(crate) modbus_slave: ModbusSlave,
    pub(crate) upper_limit_watts: usize,
    pub(crate) final_voltage_hundredth_volts: usize,
}

pub(crate) struct RD6006 {
    pub(crate) rd6006_config: RD6006Config,
    pub(crate) current_watts: usize,
}

impl RD6006Config {
    pub(crate) fn into_actor_state(
        &self,
        regulating_actor_configuration: &RegulatingActorConfiguration,
    ) -> ActorState {
        let regulator = Box::new(RD6006 {
            rd6006_config: self.clone(),
            current_watts: 0,
        });
        ActorState {
            wait_until: None,
            time_until_effective_seconds: regulating_actor_configuration
                .time_until_effective_seconds,
            special_state: ActorStateType::Regulating(RegulatingActorState {
                regulator: regulator,
                max_power: regulating_actor_configuration.max_power,
            }),
        }
    }
}

#[async_trait::async_trait]
impl Regulator for RD6006 {
    async fn change_power(&mut self, power: isize) {
        let target = power;

        let target = (target + self.current_watts as isize) as isize;
        let target = max(0, target) as usize;

        let target = min(target, self.rd6006_config.upper_limit_watts);

        let slave = self.rd6006_config.modbus_slave.clone();

        let final_voltage_hundredth_volts = self.rd6006_config.final_voltage_hundredth_volts;

        let current_milliamperes =
            power * 1000 / (self.rd6006_config.final_voltage_hundredth_volts * 10) as isize;

        task::spawn(async move {
            let ctx = slave.connect().await.unwrap();
            let mut rd6006 = RD6006Device::from_context(ctx);
            if target <= 0 {
                let _ = rd6006.turn_output_off().await;
            } else {
                let _ = rd6006
                    .set_constant_current((current_milliamperes * 1000) as u16)
                    .await;
                let _ = rd6006.set_constant_current_mode(true).await;
                let _ = rd6006
                    .set_voltage_protection(final_voltage_hundredth_volts as u16)
                    .await;
                let _ = rd6006.turn_output_on().await;
            }
        });

        self.current_watts = target;
    }

    fn power(&self) -> usize {
        self.current_watts as usize
    }
}

#[derive(Debug)]
enum RD6006Error {
    Technical,
    Domain(ExceptionCode),
}
struct RD6006Device {
    ctx: Context,
}

mod registers {
    pub static SET_CURRENT: u16 = 0x09;
    pub static SET_VOLTAGE_PROTECTION: u16 = 0x52;
    pub static SET_OUTPUT_STATE: u16 = 0x12;
    pub static SET_CONSTANT_CURRENT_MODE: u16 = 0x12;
}
impl RD6006Device {
    pub fn from_context(ctx: Context) -> Self {
        Self { ctx: ctx }
    }

    pub async fn set_constant_current(&mut self, micro_amperes: u16) -> Result<(), RD6006Error> {
        match self
            .ctx
            .write_single_register(registers::SET_CURRENT, micro_amperes)
            .await
        {
            Ok(e) => match e {
                Ok(_) => Ok(()),
                Err(c) => Err(RD6006Error::Domain(c)),
            },
            Err(_) => Err(RD6006Error::Technical),
        }
    }

    pub async fn set_constant_current_mode(&mut self, value: bool) -> Result<(), RD6006Error> {
        match self
            .ctx
            .write_single_register(
                registers::SET_CONSTANT_CURRENT_MODE,
                if value { 1 } else { 0 },
            )
            .await
        {
            Ok(e) => match e {
                Ok(_) => Ok(()),
                Err(c) => Err(RD6006Error::Domain(c)),
            },
            Err(_) => Err(RD6006Error::Technical),
        }
    }

    pub async fn set_voltage_protection(
        &mut self,
        hundredth_volts: u16,
    ) -> Result<(), RD6006Error> {
        match self
            .ctx
            .write_single_register(registers::SET_VOLTAGE_PROTECTION, hundredth_volts)
            .await
        {
            Ok(e) => match e {
                Ok(_) => Ok(()),
                Err(c) => Err(RD6006Error::Domain(c)),
            },
            Err(_) => Err(RD6006Error::Technical),
        }
    }

    pub async fn turn_output_on(&mut self) -> Result<(), RD6006Error> {
        match self
            .ctx
            .write_single_register(registers::SET_OUTPUT_STATE, 1)
            .await
        {
            Ok(e) => match e {
                Ok(_) => Ok(()),
                Err(c) => Err(RD6006Error::Domain(c)),
            },
            Err(_) => Err(RD6006Error::Technical),
        }
    }

    pub async fn turn_output_off(&mut self) -> Result<(), RD6006Error> {
        match self
            .ctx
            .write_single_register(registers::SET_OUTPUT_STATE, 0)
            .await
        {
            Ok(e) => match e {
                Ok(_) => Ok(()),
                Err(c) => Err(RD6006Error::Domain(c)),
            },
            Err(_) => Err(RD6006Error::Technical),
        }
    }
}
