# Caddy

This folder configures the reverse proxy Caddy. It uses the technique called "socket activation" to allow running Caddy
as a rootless container, while benefiting from systemd to manage it.

Inspired by the answer in https://caddy.community/t/preserving-source-ip-in-rootless-podman-network/25461/6 and the
example from https://github.com/eriksjolund/podman-caddy-socket-activation/tree/main/examples/example4

Also, each service referenced by caddy will have its own network. This reduces the risk of interference between these
services, since they cannot communicate directly.

## How to add a new app to Caddy

Say you want to expose a new app called bugabuga with the caddy server. These are the steps to take:

1. declare a new network in `~/bare/caddy/caddy_bugabuga.network` 
2. declare in `~/bare/caddy/caddy.container` that it is connected to this new network
3. declare the new service in `~/bare/caddy/Caddyfile`. You can import:
   - external_sockets to make it available to the public internet
   - internal_sockets to make it available only to the local network
   - logging to enable the default logging
   - auth to protect the service with single-sign on (should be done for all public services)
4. update the `~/bare/manual-setup.sh` to install this network and restart caddy
5. in the service compose file add something like:
  ```yaml
  services:
    bugabuga:
      networks:
        - systemd-caddy_bugabuga
  networks:
    systemd-caddy_bugabuga:
      external: true
  ```