#!/usr/bin/env bash
set -ex

# This script is not meant to be executed automatically.
# Instead, this is a scratch pad to help me remind exactly how I've setup up my system
# I've installed my home lab in Ubuntu Server 24.04
if true; then
    exit 0
fi

###
### Ethernet
###
(
sudo tee /etc/netplan/50-cloud-init.yaml << 'EOF'
network:
  version: 2
  ethernets:
    enp3s0:
      dhcp4: true
EOF

sudo chmod 600 /etc/netplan/50-cloud-init.yaml

sudo netplan apply
)

# Install Rust
sudo apt-get install -y build-essential
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

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
DEVICE=/dev/disk/by-uuid/0b6fd7a0-ceb6-488d-ae89-835ab359c887
sudo cryptsetup luksFormat "$DEVICE"
sudo cryptsetup open "$DEVICE" temp
sudo mkfs.ext4 /dev/mapper/temp
sudo cryptsetup close temp

DEVICE=/dev/sdc
sudo cryptsetup luksFormat "$DEVICE"
sudo cryptsetup open "$DEVICE" temp
sudo mkfs.ext4 /dev/mapper/temp
sudo cryptsetup close temp
)

###
### Write the encryption password on the encrypted disk itself
###
(
sudo "$HOME/home-lab/config/mount-protected.sh"
PASSWORD_FILE="$HOME/protected/backup-password.txt"
nano "$PASSWORD_FILE" # enter password
ENCRYPTION_KEY=$(cat "$PASSWORD_FILE")
echo -n "$ENCRYPTION_KEY" > "$PASSWORD_FILE" # to remove trailing new line
chmod 0600 "$PASSWORD_FILE"
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

# Allow overcommit (required by Redis)
sudo tee /etc/sysctl.d/sitegui-redis.conf << 'EOF'
vm.overcommit_memory = 1
EOF
sudo sudo sysctl --system

# Allow user services to run even after login shell session is closed
loginctl enable-linger sitegui
)

###
### LLDAP
###
(
mkdir "$HOME/bare/lldap"
cd "$HOME/bare/lldap"

openssl rand -hex 16 > secrets/jwt_secret
openssl rand -hex 16 > secrets/key_seed
openssl rand -hex 16 > secrets/ldap_user_pass
)

###
### Next cloud
###
(
mkdir "$HOME/protected/nextcloud"
cd "$HOME/protected/nextcloud"

tee secrets.conf << EOF
DATABASE_PASSWORD=$(openssl rand -hex 16)
FULLTEXTSEARCH_PASSWORD=$(openssl rand -hex 16)
IMAGINARY_SECRET=$(openssl rand -hex 16)
NEXTCLOUD_PASSWORD=$(openssl rand -hex 16)
ONLYOFFICE_SECRET=$(openssl rand -hex 16)
RECORDING_SECRET=$(openssl rand -hex 16)
REDIS_PASSWORD=$(openssl rand -hex 16)
SIGNALING_SECRET=$(openssl rand -hex 16)
TALK_INTERNAL_SECRET=$(openssl rand -hex 16)
TURN_SECRET=$(openssl rand -hex 16)
WHITEBOARD_SECRET=$(openssl rand -hex 16)
EOF

chmod 600 secrets.conf

cd "$HOME/home-lab"
cargo run -- compile-nextcloud-units \
  --input-secrets "$HOME/protected/nextcloud/secrets.conf" \
  --output-secrets-dir "$HOME/protected/nextcloud/secrets" \
  --volumes-dir "$HOME/protected/nextcloud/volumes" \
  --profiles collabora,imaginary,fulltextsearch
)

###
### Updates
###
(
# Configure the default daily updates to only consider security updates and to restart the machine if needed.

sudo tee /etc/apt/apt.conf.d/99sitegui-always-update << 'EOF'
APT::Periodic::Update-Package-Lists "always";
APT::Periodic::Unattended-Upgrade "always";
EOF

sudo tee /etc/apt/apt.conf.d/99sitegui-security-only << 'EOF'
#clear Unattended-Upgrade::Allowed-Origins;
Unattended-Upgrade::Allowed-Origins {
	"${distro_id}:${distro_codename}-security";
}
EOF

sudo tee /etc/apt/apt.conf.d/99sitegui-reboot << 'EOF'
Unattended-Upgrade::Automatic-Reboot "true";
Unattended-Upgrade::Automatic-Reboot-Time "05:00";
EOF

sudo systemctl stop apt-daily.timer
sudo systemctl disable apt-daily.timer
sudo systemctl stop apt-daily-upgrade.timer
sudo systemctl disable apt-daily-upgrade.timer
)