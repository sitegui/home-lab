#!/usr/bin/env bash
set -ex
cryptsetup open /dev/disk/by-uuid/0b6fd7a0-ceb6-488d-ae89-835ab359c887 backup-1 --key-file /home/sitegui/protected/backup-password.txt
mount /dev/mapper/backup-1 /home/sitegui/backup-1