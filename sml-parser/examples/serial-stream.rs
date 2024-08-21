use hackdose_sml_parser::message_stream::sml_message_stream;
use tokio::io::AsyncRead;
use tokio_serial::SerialStream;
use tokio_stream::StreamExt;

pub(crate) fn uart_ir_sensor_data_stream() -> impl AsyncRead {
    let ttys_location = "/dev/ttyUSB0";
    let serial = tokio_serial::new(ttys_location, 9600);
    let stream = SerialStream::open(&serial).unwrap();
    stream
}

#[tokio::main(worker_threads = 2)]
async fn main() {
    let uart = uart_ir_sensor_data_stream();
    let mut stream = sml_message_stream(uart);
    while let Some(event) = stream.next().await {
        println!("Event: {:?}", event);
    }
}
