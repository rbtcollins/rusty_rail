system:
  network.system:
  - retain_settings: True

hn0.0:
  network.managed:
  - type: alias
  - ipaddr: 10.0.0.{{ salt['rr.hostnumber'](grains['id']) }}
  - netmask: 255.255.255.0

gre0:
  network.managed:
  - type: gre
  - ipaddr: 172.16.0.1
  - netmask: 255.255.255.252
  - peer_inner_addr: 172.16.0.2
  - tunnel_addr: 10.0.0.{{ salt['rr.hostnumber'](grains['id']) }}
# NB: sharing this address over all clients as we're not going to generate
# traffic from it, and the server endpoint is hardcoded.
  - tunnel_peer: 192.168.137.188
# Sending traffic to the load balancer node

routes:
  network.routes:
    - name: hn0
    - routes:
      - name: via_lb_1
        ipaddr: 10.1.0.1
        netmask: 255.255.255.255
        gateway: 172.16.0.2

netperf:
  pkg.installed:
    - name: netperf
