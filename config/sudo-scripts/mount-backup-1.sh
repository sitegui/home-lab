#!/usr/bin/env bash
set -ex
cryptsetup open /dev/disk/by-uuid/6f20fbbb-36ba-47dc-87b1-8ef44cdef7d3 backup-1 --key-file /home/sitegui/protected/backup-password.txt
mount /dev/mapper/backup-1 /home/sitegui/backup-1