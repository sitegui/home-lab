#!/usr/bin/env bash
set -ex
mv /etc/apt/apt.conf.d/99sitegui-security-only.disabled /etc/apt/apt.conf.d/99sitegui-security-only || true
systemctl start apt-daily.service
systemctl start apt-daily-upgrade.service