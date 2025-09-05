use std::cmp::{max, min};

use tokio_modbus::{client::tcp, Slave};

use crate::actors::PowerSwitch;
use tokio_modbus::prelude::*;

mod marstek_registers {
    pub (super) const STATE: u16 = 0xa41a;
    pub (super) const FORCIBLE_CHARGE_WATTS: u16 = 0xa424;
    pub (super) const FORCIBLE_DISCHARGE_WATTS: u16 = 0xa425;

    pub (super) const STATE_CHARGE: u16 = 1;
    pub (super) const STATE_DISCHARGE: u16 = 2;

}

pub(crate) struct MarstekCharge {
    pub(crate) url: String,
    pub(crate) upper_limit_watts: usize,
    pub(crate) current_watts: isize,
}

#[async_trait::async_trait]
impl PowerSwitch for MarstekCharge {
    async fn on(&mut self) {}

    async fn off(&mut self) {}

    async fn set_power(&mut self, power: isize) {
        let power = -power;

        let target = power + self.current_watts as isize;
        let target = max(0, target);

        let target = min(target, self.upper_limit_watts as isize);
        let socket_addr = self.url.parse().unwrap();
        let slave = Slave(1);

        let mut ctx = tcp::connect_slave(socket_addr, slave).await.unwrap();
        let _ = ctx.write_single_register(marstek_registers::STATE, marstek_registers::STATE_CHARGE).await;
        let _ = ctx.write_single_register(marstek_registers::FORCIBLE_CHARGE_WATTS, target as u16).await;

        self.current_watts = target;
    }
}

pub(crate) struct MarstekDischarge {
    pub(crate) url: String,
    pub(crate) upper_limit_watts: usize,
    pub(crate) current_watts: isize,
}

#[async_trait::async_trait]
impl PowerSwitch for MarstekDischarge {
    async fn on(&mut self) {}

    async fn off(&mut self) {}

    async fn set_power(&mut self, power: isize) {
        let target = power + self.current_watts as isize;
        let target = max(0, target);

        let target = min(target, self.upper_limit_watts as isize);

        let socket_addr = self.url.parse().unwrap();
        let slave = Slave(1);

        let mut ctx = tcp::connect_slave(socket_addr, slave).await.unwrap();
        let _ = ctx.write_single_register(marstek_registers::STATE, marstek_registers::STATE_DISCHARGE).await;
        let _ = ctx.write_single_register(marstek_registers::FORCIBLE_DISCHARGE_WATTS, target as u16).await;

        self.current_watts = target;
    }
}
