# Rusty rail - a line rate distributed load balancer

Rusty rail is inspired by the [Google Maglev
paper](http://research.google.com/pubs/pub44824.html)

# Current status

Bridges a single interface in full passthrough mode to preserve connections to
host, and directs all GRE packets back out the wire interface.

# Installation

Rusty rail uses [Netmap](https://github.com/luigirizzo/netmap) - install that
first.

Netmap transmitted packets don't get offloaded checksums processed, so while
running Rusty rail offload must be disabled on the interfaces in use:

```
sudo ethtool -K eth0 tx off rx off gro off tso off gso off
```

Rusty rail is written in [Rust](https://www.rust-lang.org/) so you will need a
working installation, and [Cargo](http://doc.crates.io/index.html) is highly
recommended to manage the build process.

# Quickstart

```
git clone XXX
cargo build
sudo ethtool -K eth0 tx off rx off gro off tso off gso off
sudo insmod netmap
# sudo RUST_BACKTRACE=1 RR_DEVICE=eth0 RR_TARGET_IPS=10.1.0.1 target/debug/rusty_rail
sudo RUST_BACKTRACE=1 RR_DEVICE=eth0 RR_TARGET_IPS="server1ip;server2ip" target/debug/rusty_rail
```

# Configuration

Configuration is via environment variables.

* ``RR_DEVICE`` should be the name of the interface to receive and transmit GRE
  wrapped packets on.

# Testing

## Test bed overview

The initial deployment architecture we want to emulate is:

Clients -> I/N -> router [GRE encap] -> LoadBalancers -> Servers [GRE decap, DSR]

Key characteristics:
 - servers need GRE 1/2 sided, and the route back to the client src IP must not
   be within the tunnel network
 - clients need to be able to send packets to the loadbalancers that are GRE

So - minimal setup:
 - V is the network of your hypervisor virtual network
 - pick T not in V as the subnet for traffic within GRE (used for e.g. ICMP
   originating from the GRE receiver) Suggest a /22 (net, peer1, peer2, broad)
 - pick C not in V,T as the client src IPs
 - pick S not in V,T or C as the service src IP [decoupled from server count as
   this is the virtual IP for the service]
 - disable anti-spoofing in your hypervisor network
 - add one (or more) addresses from C to the client workload VMs
 - add one (or more if you're running more than one test service) address from
   S to the server workload VMs
 - have a tunnel from C to the load balancer VM(s) over GRE for traffic destined for S
 - disable return path spoof protection on the clients (as traffic from S will
   arrive unencaped)
 - have a tunnel from LB to S on the server VMs for traffic destined to S
 - possibly disable the outbound route matching?

iteration 1:
 one client, one server, one LB
 V: 192.168.137.0/24
 T(client): 172.16.0.1/22 
 T(server): 172.16.0.2/22
 C: 10.0.0.0/24
   C1: 10.0.0.1
 S: 10.1.0.0/24
   S1: 10.1.0.1
iteration 2:
 one client, two servers, one LB
iteration 3:
 one client, two servers, two LB
iteration 4:
 two clients, two servers, two LB

Test automation: salt. Just because.

## Install a salt master

Anywhere you like. Have fun. Knock yourself out. The states for this project
are in salt/, so add this path to your file_roots in /etc/salt/master (or
wherever your master conf is).

## Test node prep

Test nodes - http://ftp.freebsd.org/pub/FreeBSD/releases/VM-IMAGES/11.0-RELEASE/amd64/Latest/FreeBSD-11.0-RELEASE-amd64.vhd.xz

See https://blogs.msdn.microsoft.com/kylie/2014/12/25/running-freebsd-on-hyper-v/ ; gen 1, dynamic memory off. Give 256M to each VM.

username root; no password
change password e.g. foo

install ssh for mgmt:
vi /etc/rc.conf

```sshd_enable=YES```

vi /etc/ssh/sshd_config
enable root logins

then:
```sh /etc/rc.d/sshd start```

ssh-copy-key as desired
disable challenge-response logins to disable passwords

Enable GRE:
echo if_gre_load="YES" > /boot/loader.conf

reboot and check everything came up ok.
kldstat

Install salt:
```
pkg install py27-salt
cat << EOF > /usr/local/etc/salt/minion
master: $masterip
id: $uniqueid
minion_id_caching: False
grains:
  roles:
    - unassigned
EOF
sysrc salt_minion_enable="YES"
rm -fr /usr/local/etc/salt/pki/minion
rm -fr /usr/local/etc/salt/minion_id
```

Clone the VM at this point to permit rapid creation of additional machines.

## Per node
1. Boot the node.
2. Change the hostname
# Avoid salt crashing on start - or hand this out via DHCP etc.
sysrc hostname="$ROLE-$N.local"
hostname $ROLE-$N.local
2. restart salt:
   ```service salt_minion start```



# License

Apache-2.0.
