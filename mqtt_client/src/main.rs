use clap::Parser;
use hackdose_sml_parser::application::domain::Scale;
use hackdose_sml_parser::application::obis;
use hackdose_sml_parser::{
    application::{
        domain::{AnyValue, SmlMessageEnvelope, SmlMessages},
        obis::Obis,
    },
    message_stream::sml_message_stream,
};
use options::Args;
use rumqttc::{v5::mqttbytes::QoS, v5::AsyncClient, v5::MqttOptions};
use std::time::Duration;
use tokio::io::AsyncRead;
use tokio_serial::SerialStream;
use tokio_stream::StreamExt;

mod options;

pub(crate) fn uart_ir_sensor_data_stream(ttys_location: String) -> impl AsyncRead {
    let serial = tokio_serial::new(ttys_location, 9600);
    let stream = SerialStream::open(&serial).unwrap();
    stream
}

#[tokio::main(worker_threads = 2)]
async fn main() {
    let args = Args::parse();
    let config = args.get_config_file().await;

    let uart = uart_ir_sensor_data_stream(config.ttys_location);
    let mut stream = sml_message_stream(uart);

    let mut mqttoptions = MqttOptions::new(
        "rumqtt-async",
        config.mqtt_broker.host,
        config.mqtt_broker.port,
    );
    mqttoptions.set_credentials(config.mqtt_broker.username, config.mqtt_broker.password);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    tokio::spawn(async move {
        loop {
            let _ = eventloop.poll().await;
        }
    });

    while let Some(event) = stream.next().await {
        for publication in &config.publications {
            let v = find_value(&event, &publication.obis);
            if let Some(v) = v {
                let formatted = format!("{:?}", v);
                let res = client
                    .publish(
                        publication.topic.clone(),
                        QoS::AtLeastOnce,
                        false,
                        formatted,
                    )
                    .await;
            }
        }
    }
}

pub fn find_value(messages: &SmlMessages, obis_value: &Obis) -> Option<i32> {
    for list in &messages.messages {
        match list {
            SmlMessageEnvelope::GetOpenResponse(_) => continue,
            SmlMessageEnvelope::GetListResponse(body) => {
                let values = &body.value_list;
                let identified = values
                    .iter()
                    .flat_map(|value| {
                        Obis::from_number(&value.object_name)
                            .map(|x| (x, value.value.clone(), value.scaler.clone()))
                    })
                    .collect::<Vec<_>>();

                let usage = identified
                    .iter()
                    .find(|(o, _, _)| o == obis_value)
                    .map(|(_, v, scaler)| v.scale(scaler.unwrap_or(0)));

                if let Some(usage) = usage {
                    if let AnyValue::Signed(value) = usage {
                        return Some(value as i32);
                    }
                    if let AnyValue::Unsigned(value) = usage {
                        return Some(value as i32);
                    }
                }
            }
            SmlMessageEnvelope::GetCloseResponse => continue,
        }
    }
    return None;
}

// Todo:
// - cli
// - compile on github
// issue: create configuration file as an alternative for cli options
