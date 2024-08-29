# Hackdose mqtt Client

This little piece of Software lets you publish the data from your Smart Meter to arbtitrary
mqtt brokers (i.e. home assistant).

## Quick Start

 - get the appropriate binary `mqtt-client-<your hardware here>` from [releases](https://github.com/torfmaster/hackdose/actions/workflows/release.yaml)
 - copy it to your machine (to `/home/pi`, otherwise you need to adapt the `.service` file below)
 - create a configuration file (see sample) and move it to `/home/pi/hackdose_mqtt_client_config.yaml`
 - enable the client in systemd
    - copy `hackdose_mqtt.service` to `/etc/systemd/system`
    - `systemctl enable hackdose_mqtt.service`
    - `systemctl start hackdose_mqtt.service`

## Compiling the client yourself

 - install the cross-compilation target for your architecture
   `armv7-unknown-linux-musleabihf`
 - compile the binary using `cargo build --target=armv7-unknown-linux-musleabihf --release` 
