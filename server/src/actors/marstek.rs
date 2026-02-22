use std::cmp::{max, min};

use serde::{Deserialize, Serialize};
use tokio::task;

use crate::{actors::Regulator, config::ModbusSlave};
use tokio_modbus::prelude::*;

mod marstek_registers {
    pub(super) const STATE: u16 = 0xa41a;
    pub(super) const FORCIBLE_CHARGE_WATTS: u16 = 0xa424;
    pub(super) const FORCIBLE_DISCHARGE_WATTS: u16 = 0xa425;

    pub(super) const STATE_CHARGE: u16 = 1;
    pub(super) const STATE_DISCHARGE: u16 = 2;
}

#[derive(Serialize, Deserialize, Clone)]

pub(crate) struct MarstekConfiguration {
    pub(crate) modbus_slave: ModbusSlave,
    pub(crate) upper_limit_watts: usize,
}

pub(crate) struct MarstekCharge {
    pub(crate) modbus_slave: ModbusSlave,
    pub(crate) upper_limit_watts: usize,
    pub(crate) current_watts: usize,
}

#[async_trait::async_trait]
impl Regulator for MarstekCharge {
    async fn change_power(&mut self, power: isize) {
        println!(
            "Change power of Marstek Charge by {} current power {}",
            power, self.current_watts
        );
        let target = power;

        let target = (target + self.current_watts as isize) as isize;
        let target = max(0, target) as usize;

        let target = min(target, self.upper_limit_watts);

        println!("Setting Marstek Charge to power of {}", target);

        let slave = self.modbus_slave.clone();

        task::spawn(async move {
            let mut ctx = slave.connect().await.unwrap();
            let _ = ctx
                .write_single_register(marstek_registers::STATE, marstek_registers::STATE_CHARGE)
                .await;
            let _ = ctx
                .write_single_register(marstek_registers::FORCIBLE_CHARGE_WATTS, target as u16)
                .await;
        });

        self.current_watts = target;
    }

    fn power(&self) -> usize {
        self.current_watts as usize
    }
}

pub(crate) struct MarstekDischarge {
    pub(crate) modbus_slave: ModbusSlave,
    pub(crate) upper_limit_watts: usize,
    pub(crate) current_watts: usize,
}

#[async_trait::async_trait]
impl Regulator for MarstekDischarge {
    async fn change_power(&mut self, power: isize) {
        println!(
            "Change power of Marstek Discharge by {} current power {}",
            power, self.current_watts
        );
        // Consumption (positive values) should be produced by the actor
        let target = power;

        let target = target + self.current_watts as isize;
        let target = max(0, target) as usize;

        let target = min(target, self.upper_limit_watts);

        println!("Setting Marstek Discharge to power of {}", target);

        self.current_watts = target;

        let modbus_slave = self.modbus_slave.clone();

        task::spawn(async move {
            let mut ctx = modbus_slave.connect().await.unwrap();
            let _ = ctx
                .write_single_register(marstek_registers::STATE, marstek_registers::STATE_DISCHARGE)
                .await;
            let _ = ctx
                .write_single_register(marstek_registers::FORCIBLE_DISCHARGE_WATTS, target as u16)
                .await;
        });
    }

    fn power(&self) -> usize {
        self.current_watts as usize
    }
}
