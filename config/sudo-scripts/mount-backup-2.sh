#!/usr/bin/env bash
set -ex
cryptsetup open /dev/disk/by-uuid/ea35bd94-9e4f-4d5f-92b3-a48e3a6213e2 backup-2 --key-file /home/sitegui/protected/backup-password.txt
mount /dev/mapper/backup-2 /home/sitegui/backup-2