#!/bin/bash
set -eux

pushd ../app
trunk build --release
popd

source profiles/$PROFILE_NAME/.env
cargo build --target=$TARGET --release
sshpass -p $PI_PASSWORD  scp -C -P$PI_SSH_PORT hackdose.service $PI_USER@$PI_HOST:/etc/systemd/system
sshpass -p $PI_PASSWORD ssh -p $PI_SSH_PORT $PI_USER@$PI_HOST 'systemctl enable hackdose.service'
sshpass -p $PI_PASSWORD ssh -p $PI_SSH_PORT $PI_USER@$PI_HOST 'systemctl stop hackdose.service'
sshpass -p $PI_PASSWORD  scp -C -P$PI_SSH_PORT ../target/$TARGET/release/hackdose-server $PI_USER@$PI_HOST:/usr/bin/ 
sshpass -p $PI_PASSWORD  scp -C -P$PI_SSH_PORT profiles/$PROFILE_NAME/hackdose.yaml $PI_USER@$PI_HOST:/etc/
sshpass -p $PI_PASSWORD  ssh -p $PI_SSH_PORT $PI_USER@$PI_HOST 'systemctl start hackdose.service'
