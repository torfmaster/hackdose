use serde::{Deserialize, Serialize};
use tokio_modbus::{
    client::{rtu, tcp, Context},
    Slave,
};
use tokio_serial::SerialStream;

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct RtuModbusConnectionData {
    ttys_location: String,
    baud_rate: u32,
    stop_bits: StopBits,
    parity: Parity,
    flow_control: FlowControl,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) enum StopBits {
    One,
    Two,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) enum Parity {
    Even,
    Odd,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) enum FlowControl {
    Software,
    Hardware,
    None,
}

#[derive(Serialize, Deserialize, Clone)]

pub(crate) struct ModbusSlave {
    slave_adress: u8,
    modbus_connection: ModbusConnection,
}
#[derive(Serialize, Deserialize, Clone)]
pub(crate) enum ModbusConnection {
    TCP(String),
    RTU(RtuModbusConnectionData),
}

#[derive(Debug)]
pub(crate) enum ModbusError {
    ConnectionError,
}

impl ModbusSlave {
    pub(crate) async fn connect(&self) -> Result<Context, ModbusError> {
        match &self.modbus_connection {
            ModbusConnection::TCP(url) => {
                let socket_addr = url.parse().unwrap();
                let slave = Slave(1);
                tcp::connect_slave(socket_addr, slave)
                    .await
                    .map_err(|_| ModbusError::ConnectionError)
            }
            ModbusConnection::RTU(config) => {
                let slave = Slave(1);

                let stop_bits = match config.stop_bits {
                    StopBits::One => tokio_serial::StopBits::One,
                    StopBits::Two => tokio_serial::StopBits::Two,
                };
                let parity = match &config.parity {
                    Parity::Even => tokio_serial::Parity::Even,
                    Parity::Odd => tokio_serial::Parity::Odd,
                };

                let flow_control = match &config.flow_control {
                    FlowControl::Software => tokio_serial::FlowControl::Software,
                    FlowControl::Hardware => tokio_serial::FlowControl::Hardware,
                    FlowControl::None => tokio_serial::FlowControl::None,
                };
                let builder = tokio_serial::new(config.ttys_location.clone(), config.baud_rate)
                    .stop_bits(stop_bits)
                    .flow_control(flow_control)
                    .parity(parity);
                let port =
                    SerialStream::open(&builder).map_err(|_| ModbusError::ConnectionError)?;

                Ok(rtu::attach_slave(port, slave))
            }
        }
    }
}
