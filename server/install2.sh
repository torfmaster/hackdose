#!/bin/bash
set -eux

pushd ../app
trunk build --release
popd

source profiles/$PROFILE_NAME/.env
cargo build --target=$TARGET --release
sshpass -p $PI_PASSWORD  scp -C -P$PI_SSH_PORT hackdose2.service $PI_USER@$PI_HOST:/etc/systemd/system
sshpass -p $PI_PASSWORD ssh -p $PI_SSH_PORT $PI_USER@$PI_HOST 'systemctl enable hackdose2.service'
sshpass -p $PI_PASSWORD ssh -p $PI_SSH_PORT $PI_USER@$PI_HOST 'systemctl stop hackdose2.service'
sshpass -p $PI_PASSWORD  scp -C -P$PI_SSH_PORT ../target/$TARGET/release/hackdose-server $PI_USER@$PI_HOST:/usr/bin/hackdose-server2
sshpass -p $PI_PASSWORD  scp -C -P$PI_SSH_PORT profiles/$PROFILE_NAME/hackdose.yaml $PI_USER@$PI_HOST:/etc/hackdose2.yaml
sshpass -p $PI_PASSWORD  ssh -p $PI_SSH_PORT $PI_USER@$PI_HOST 'systemctl start hackdose2.service'
