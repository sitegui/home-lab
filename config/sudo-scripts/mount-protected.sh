#!/usr/bin/env bash
set -ex
cryptsetup open /dev/disk/by-uuid/53d44bd7-62eb-4e9c-875b-1068a2bc95af protected
mount /dev/mapper/protected /home/sitegui/protected