#!/usr/bin/env bash

set -ex

cargo build --release
scp ./target/release/home-lab sitegui@192.168.1.51:/home/sitegui/home-lab
# shellcheck disable=SC2029
ssh sitegui@192.168.1.51 ./home-lab "$@"