# Nextcloud

This installation is based on the
official [manual installation instructions](https://github.com/nextcloud/all-in-one/tree/main/manual-install). They use
docker, compose and named volumes. To fit my setup better, I've decided to use podman, systemd unit files and bind
volumes:

- podman implements a rootless approach, which I prefer
- systemd unit files are easier to manage globally with all my other services on the server
- bind volumes are easier to integrate with my global backup solution

The files `latest.yml` and `sample.conf` were copied from the most recent commit at the time. I've written a script to
generate the necessary `*.container` files, focusing on facilitating future updates.

