use std::{collections::HashMap, sync::Arc};

use chrono::Utc;
use hackdose_server_shared::DataPoint;
use hackdose_sml_parser::application::{
    domain::{AnyValue, SmlMessages},
    obis::Obis,
};
use tokio::sync::{mpsc::Sender, Mutex};
use tokio_stream::Stream;
use tokio_stream::StreamExt;

use crate::{data::EnergyData, smart_meter::body::find_watts};

pub(crate) async fn handle_power_events(
    tx: &mut Sender<i32>,
    mutex: Arc<Mutex<HashMap<Obis, AnyValue>>>,
    mut power_events: impl Stream<Item = SmlMessages> + Unpin + Send + 'static,
    mut energy_data: EnergyData,
) {
    while let Some(message) = power_events.next().await {
        let watts = find_watts(&message, mutex.clone()).await;

        match watts {
            Some(watts) => {
                let data = DataPoint {
                    date: Utc::now(),
                    value: watts,
                };
                energy_data.put(data).await;
                energy_data.log_data(data).await;

                tx.send(watts).await.unwrap();
            }
            None => {}
        }
    }
}
