#!/usr/bin/env bash
set -ex

# This script is not meant to be executed automatically.
# Instead, this is a scratch pad to help me remind exactly how I've setup up my system
# I've installed my home lab in Ubuntu Server 24.04
if true; then
    exit 0
fi

###
### Base folders
###
(
# The "bare" folder is not encrypted. It contains everything that can run automatically, but will not be protected
# against access after a physical stealing of this material from my home.
mkdir "$HOME/bare"

# The "protected" folder is encrypted and must be manually unlocked before the services inside can start.
# Note that, in the backup disks, both bare and protected folders will be copied and encrypted. The sole distinction
# is in the main server, in which only the protected folder is encrypted.
mkdir "$HOME/protected"

# The "backup-1" and "backup-2" folders are used by two different disks that will be mounted for the regular backups
mkdir "$HOME/backup-1" "$HOME/backup-2"
)

###
### Create a new encrypted disk for backups
###
(
DEVICE=/dev/disk/by-uuid/746cc868-91bc-4f4d-ad78-2fc00084bcad
sudo cryptsetup luksFormat "$DEVICE"
sudo cryptsetup open "$DEVICE" temp
sudo mkfs.ext4 /dev/mapper/temp
sudo cryptsetup close temp
)

###
### Scripts to mount and unmount the protected disks
###
# In order to do these operations, the scripts need super user access. In order to reduce the surface of attack, they:
# - avoid all uses of complex bash features
# - are owned by root
# - can be executed by the user as sudo without entering password
(
function allow_sudo_without_password() {
  SCRIPT="$1"

  echo "sitegui ALL=(ALL) NOPASSWD: $SCRIPT" | sudo tee --append /etc/sudoers.d/sitegui

  # Protect the file so that only root can modify it
  sudo chown root:root "$SCRIPT"
  sudo chmod 744 "$SCRIPT"
}

allow_sudo_without_password "$HOME/bare/mount-protected.sh"
allow_sudo_without_password "$HOME/bare/mount-backup-1.sh"
allow_sudo_without_password "$HOME/bare/umount-backup-1.sh"
)

###
### Write the encryption password on the encrypted disk itself
###
(
sudo "$HOME/bare/mount-protected.sh"
PASSWORD_FILE="$HOME/protected/backup-password.txt"
nano "$PASSWORD_FILE" # enter password
ENCRYPTION_KEY=$(cat "$PASSWORD_FILE")
echo -n "$ENCRYPTION_KEY" > "$PASSWORD_FILE" # to remove trailing new line
chmod 0600 "$PASSWORD_FILE"
)

###
### Automatic backups
###
(
SYSTEMD_USER="$HOME/.config/systemd/user"
mkdir -p "$SYSTEMD_USER"

tee "$SYSTEMD_USER/backup.service" << 'EOF'
[Unit]
Description=Backup

[Service]
ExecStart=%h/bare/home-lab backup
Restart=on-failure
RestartSec=300

[Install]
WantedBy=multi-user.target
EOF

tee "$SYSTEMD_USER/backup.timer" << 'EOF'
[Unit]
Description=Backup timer

[Timer]
OnCalendar=*-*-* 02:00:00
Persistent=true

[Install]
WantedBy=timers.target
EOF

systemctl --user daemon-reload
systemctl --user enable backup.timer
systemctl --user start backup.timer
)

###
### Don't sleep on lid close
###
# From https://askubuntu.com/questions/15520/how-can-i-tell-ubuntu-to-do-nothing-when-i-close-my-laptop-lid#372616 and
# https://askubuntu.com/questions/141866/keep-ubuntu-server-running-on-a-laptop-with-the-lid-closed
(
sudo mkdir --parents /etc/systemd/logind.conf.d

sudo tee /etc/systemd/logind.conf.d/sitegui.conf << 'EOF'
[Login]
HandleLidSwitch=ignore
HandleLidSwitchDocked=ignore
HandleLidSwitchExternalPower=ignore
EOF

sudo systemctl restart systemd-logind
)

###
### Switch to use zshell
###
(
sudo apt-get install -y zsh
chsh -s "$(which zsh)"
)

###
### Firewall
###
# Since docker does not play well with ufw, I'll use podman.
(
sudo ufw default deny
# Only allow SSH from local network, using the local IPv4
sudo ufw allow from 192.168.1.0/24 to any port 22
# Only allow administrative services at port 8080 to be accessed from local IPv4 and IPv6
sudo ufw allow from 192.168.1.0/24 to any port 8080
sudo ufw allow from 2a01:e0a:82c:b1f0::/64 to any port 8080
# Allow public access to ports 80 and 443
sudo ufw allow 80
sudo ufw allow 443
sudo ufw enable
)

###
### Podman
###
(
sudo apt-get -y install podman podman-compose

# Allow non-root users to listen to port 80 (and above)
sudo tee /etc/sysctl.d/sitegui-podman.conf << 'EOF'
net.ipv4.ip_unprivileged_port_start=80
EOF
sudo sudo sysctl --system

# Allow user services to run even after login shell session is closed
loginctl enable-linger sitegui
)

###
### Caddy
###
(
mkdir "$HOME/bare/caddy"
cd "$HOME/bare/caddy"

mkdir config
mkdir data

CONTAINERS_SYSTEMD="$HOME/.config/containers/systemd/"
mkdir -p "$CONTAINERS_SYSTEMD"
ln --force --symbolic "$(pwd)"/caddy.container "$CONTAINERS_SYSTEMD"
ln --force --symbolic "$(pwd)"/caddy.network "$CONTAINERS_SYSTEMD"
ln --force --symbolic "$(pwd)"/caddy_lldap.network "$CONTAINERS_SYSTEMD"
ln --force --symbolic "$(pwd)"/caddy_authelia.network "$CONTAINERS_SYSTEMD"
ln --force --symbolic "$(pwd)"/caddy_jellyfin.network "$CONTAINERS_SYSTEMD"

SYSTEMD_USER="$HOME/.config/systemd/user/"
mkdir -p "$SYSTEMD_USER"
ln --force --symbolic "$(pwd)"/caddy.socket "$SYSTEMD_USER"

systemctl --user daemon-reload
systemctl --user restart caddy.service
)

###
### LLDAP
###
(
mkdir "$HOME/bare/lldap"
cd "$HOME/bare/lldap"

mkdir secrets
mkdir data

openssl rand -hex 16 > secrets/jwt_secret
openssl rand -hex 16 > secrets/key_seed
openssl rand -hex 16 > secrets/ldap_user_pass

ln --force --symbolic "$(pwd)"/lldap.service "$HOME/.config/systemd/user/"

systemctl --user daemon-reload
systemctl --user enable lldap.service
systemctl --user restart lldap.service
)

###
### Authelia
###
(
# Go to https://ldap.sitegui.dev:8080/users/create and create a new user with password from the password file
# `ldap_password` below, then add it to the group `lldap_password_manager`

mkdir "$HOME/bare/authelia"
cd "$HOME/bare/authelia"

mkdir secrets
mkdir data

openssl rand -hex 16 > secrets/ldap_password
openssl rand -hex 16 > secrets/jwt_secret
openssl rand -hex 16 > secrets/session_secret
openssl rand -hex 16 > secrets/encryption_key

CONTAINERS_SYSTEMD="$HOME/.config/containers/systemd/"
ln --force --symbolic "$(pwd)"/authelia_redis.network "$CONTAINERS_SYSTEMD"

ln --force --symbolic "$(pwd)"/authelia.service "$HOME/.config/systemd/user/"
ln --force --symbolic "$(pwd)"/redis/authelia_redis.service "$HOME/.config/systemd/user/"

systemctl --user daemon-reload
systemctl --user enable authelia.service
systemctl --user enable authelia_redis.service
systemctl --user restart authelia.service
systemctl --user restart authelia_redis.service
)

###
### Protected services
###
# In using a custom systemd target, to let all services that depend on protected data to start once the system is
# unlocked
(
ln --force --symbolic "$HOME/bare/protected.target" "$HOME/.config/systemd/user/"

systemctl --user daemon-reload
)

###
### Jellyfin
###
(
cd "$HOME/protected/jellyfin"
ln --force --symbolic "$(pwd)/jellyfin.service" "$HOME/.config/systemd/user/"

systemctl --user daemon-reload
)