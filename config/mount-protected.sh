#!/usr/bin/env bash
set -ex
sudo cryptsetup open /dev/disk/by-uuid/0b6fd7a0-ceb6-488d-ae89-835ab359c887 protected
sudo mount /dev/mapper/protected "$HOME/protected"