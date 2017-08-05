# TODO: the mshome.net glue is because my node ids aren't correct - fix that to
# make this easy for others to use.

system:
  network.system:
  - retain_settings: True

hn0.0:
  network.managed:
  - type: alias
  - ipaddr: 10.1.0.1
  - netmask: 255.255.255.0

gre0:
  network.managed:
  - type: gre
  - ipaddr: 172.16.0.2
  - peer_inner_addr: 172.16.0.1
  - netmask: 255.255.255.252
  - tunnel_addr: server{{salt['rr.hostnumber'](grains['id'])}}.mshome.net
# nodes own hostname because the LB can't route to the non-on-net address yet
# (and perhaps we simply don't need to do that?)
#  - tunnel_addr: 10.1.0.1
# NB: tunnel_peer won't be used as we won't ever send traffic to 172.16.0.1...
# and we're not going to route the client through the tunnel since we want DSR
# - tunnel peer is the LB because the src address is filtered by the gre stack.
  - tunnel_peer: 192.168.137.188

# netsteps:
# - add a route to 10.0.0.X for each slave, via their client address
#   ... put those in a reported back value of some sort? .. dns works clientN.mshome.net
routes:
  network.routes:
    - name: hn0
    - routes:
      - name: dsr_client_1
        ipaddr: 10.0.0.1
        netmask: 255.255.255.255
        gateway: client1.mshome.net
      - name: dsr_client_2
        ipaddr: 10.0.0.2
        netmask: 255.255.255.255
        gateway: client2.mshome.net


netperf:
  pkg.installed:
    - name: netperf
