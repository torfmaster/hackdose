use gpio_cdev::{Chip, LineRequestFlags};
use tokio::io::AsyncRead;
use tokio_serial::SerialStream;

use crate::Configuration;

pub(crate) mod body;

pub(crate) fn uart_ir_sensor_data_stream(config: &Configuration) -> impl AsyncRead {
    let serial = tokio_serial::new(&config.ttys_location, 9600);
    let stream = SerialStream::open(&serial).unwrap();
    stream
}

pub(crate) fn enable_ir_sensor_power_supply(config: &Configuration) {
    let mut chip = Chip::new(&config.gpio_location).unwrap();
    let output = chip.get_line(config.gpio_power_pin).unwrap();
    let output_handle = output
        .request(LineRequestFlags::OUTPUT, 0, "mirror-gpio")
        .unwrap();

    output_handle.set_value(1).unwrap();
}
