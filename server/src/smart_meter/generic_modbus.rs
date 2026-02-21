use std::time::Duration;

use chrono::{DateTime, Utc};
use hackdose_server_shared::DataPoint;
use tokio::{
    sync::mpsc::Sender,
    time::{sleep, timeout},
};
use tokio_modbus::client::Reader;

use crate::{data::EnergyData, GenericModbusCounterSlaveData};

pub(crate) fn spawn_modbus_producer(
    config: &GenericModbusCounterSlaveData,
    tx: &mut Sender<(i32, DateTime<Utc>)>,

    mut energy_data: EnergyData,
) {
    let con = config.clone();

    let cloned_tx = tx.clone();

    tokio::spawn(async move {
        loop {
            let mut ctx = con.modbus_slave.connect().await.unwrap();
            loop {
                let v = timeout(
                    Duration::from_secs(5),
                    ctx.read_holding_registers(con.register, 1),
                )
                .await;
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
