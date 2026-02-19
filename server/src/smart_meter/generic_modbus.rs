use std::time::Duration;

use chrono::{DateTime, Utc};
use hackdose_server_shared::DataPoint;
use tokio::{
    sync::mpsc::Sender,
    time::{sleep, timeout},
};
use tokio_modbus::{
    client::{rtu, Reader},
    Slave,
};
use tokio_serial::{FlowControl, Parity, SerialStream, StopBits};

use crate::{data::EnergyData, GenericModbusSlaveData};

pub(crate) fn spawn_modbus_producer(
    config: &GenericModbusSlaveData,
    tx: &mut Sender<(i32, DateTime<Utc>)>,

    mut energy_data: EnergyData,
) {
    let con = config.clone();

    let slave = Slave(1);

    let stop_bits = match con.stop_bits {
        crate::StopBits::One => StopBits::One,
        crate::StopBits::Two => StopBits::Two,
    };
    let parity = match &con.parity {
        crate::Parity::Even => Parity::Even,
        crate::Parity::Odd => Parity::Odd,
    };

    let flow_control = match &config.flow_control {
        crate::FlowControl::Software => FlowControl::Software,
        crate::FlowControl::Hardware => FlowControl::Hardware,
        crate::FlowControl::None => FlowControl::None,
    };

    let cloned_tx = tx.clone();

    tokio::spawn(async move {
        let path = &con.ttys_location.clone();

        loop {
            let builder = tokio_serial::new(path, con.baud_rate)
                .stop_bits(stop_bits)
                .flow_control(flow_control)
                .parity(parity);
            let port = SerialStream::open(&builder).unwrap();

            let mut ctx = rtu::attach_slave(port, slave);
            loop {
                let v = timeout(Duration::from_secs(5), ctx.read_holding_registers(1, 1)).await;
                if let Ok(Ok(Ok(v))) = v {
                    match v.first() {
                        Some(v) => {
                            let date = Utc::now();
                            let v = (*v as i16) as i32;
                            let data = DataPoint { date, value: v };
                            energy_data.put(data).await;
                            energy_data.log_data(data).await;
                            let _ = cloned_tx.send((v, date)).await;
                        }
                        None => (),
                    }
                } else {
                    break;
                }
                sleep(Duration::from_millis(con.poll_intervall_millis as u64)).await;
            }
        }
    });
}
