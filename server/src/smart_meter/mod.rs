use gpio_cdev::{Chip, LineRequestFlags};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncRead;
use tokio_serial::SerialStream;

use crate::config::ModbusSlave;

pub(crate) mod body;
pub(crate) mod generic_modbus;

#[derive(Serialize, Deserialize, Clone)]
pub(crate) enum PowerMeasurement {
    SmlSmartMeter(SmlSmartMeterData),
    GenericModbusSlave(GenericModbusCounterSlaveData),
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct SmlSmartMeterData {
    gpio_location: Option<String>,
    ttys_location: String,
    gpio_power_pin: u32,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct GenericModbusCounterSlaveData {
    modbus_slave: ModbusSlave,
    // holds the current effective power
    register: u16,
    poll_intervall_millis: usize,
}

pub(crate) fn uart_ir_sensor_data_stream(config: &SmlSmartMeterData) -> impl AsyncRead {
    let serial = tokio_serial::new(&config.ttys_location, 9600);
    let stream = SerialStream::open(&serial).unwrap();
    stream
}

pub(crate) fn enable_ir_sensor_power_supply(config: &SmlSmartMeterData) {
    if let Some(gpio_location) = &config.gpio_location {
        let mut chip = Chip::new(gpio_location).unwrap();
        let output = chip.get_line(config.gpio_power_pin).unwrap();
        let output_handle = output
            .request(LineRequestFlags::OUTPUT, 0, "mirror-gpio")
            .unwrap();

        output_handle.set_value(1).unwrap();
    }
}
