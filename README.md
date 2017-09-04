# Rusty rail - a line rate distributed load balancer

Rusty rail is inspired by the [Google Maglev
paper](http://research.google.com/pubs/pub44824.html)

# Current status

Bridges a single interface in full passthrough mode to preserve connections to
host, and directs all GRE packets back out the wire interface to a consistently
hashed set of backend servers.

# Desired but unimplemented features

1. RPC/API for runtime updates: marking servers as unhealthy, and adding and
   removing servers.
2. A metrics API endpoint - /healthz style.
3. Support for discrete backend server sets for different virtual IP addresses.
4. Pluggable networking: glue into DPDK or XDP.
   e.g. have the control logic in userspace but forwarding implemented via XDP
   rather than using netmap to implement the forwarding in userspace.
5. Alternative routing strategies:
   - IPinIP
   - VXLAN or GENEVE encap
   - regular router - no GRE (and thus requires same-subnet or split routing
     tables within the infrastructure... but may make the system accessible
     with routers that cannot do other encap + ECMP spraying).
6. Learn about server status via ICMP code 3 - if the GRE datagrams cannot be
   delivered the server is clearly not available and theres no need to wait for
   monitoring to mark it dead.
7. IPv6 support.

# Installation

Rusty rail uses [Netmap](https://github.com/luigirizzo/netmap) - install that
first.

Netmap transmitted packets don't get offloaded checksums processed (at least
with the hyper-v network driver I have been testing with), so while running
Rusty rail offload must be disabled on the interfaces in use:

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
* ``RR_TARGET_IPS`` should be a ; delimited list of IP addresses to forward to.

# Deployment

Many different topologies are possible - single cluster vs multiple clusters,
same subnet for load balancers and servers vs different subnets, and possibly
the use of tromboning back out to the internet to deal with a local cluster
with no live endpoints.

The simplest deployment is to have a cluster of rusty rail nodes behind a
router, the servers serving the traffic on the same subnet, and the virtual
IP addresses that clients will use on a second subnet.

+-----+ +-----+ +-----+
| C1  | | C2  | | CN  |
+--+--+ +--+--+ +--+--+
   |       |       |
   +---------------+   Network 1: Clients
           |           Network 2: LB and Servers
        +--+---+       Network 3: Virtual IPs
        |Router|                  Destination LB1/2/3 in GRE with ECMP
        +--+---+
           |
   +---------------+-------+-------+-------+
   |       |       |       |       |       |
+--+--+ +--+--+ +--+--+ +--+--+ +--+--+ +--+--+
| LB1 | | LB2 | | LBN | | S1  | | S2  | | SN  |
+-----+ +-----+ +-----+ +-----+ +-----+ +-----+

The traffic flow for a single stream will go from the client to the router,
be GRE encapsulated there, forwarded to one of the rusty rail nodes, forwarded
from there (still GRE encapsulated) to a server, where the network stack on the
server will decapsulate the packets and process the inner packet originally
encapsulated by the router locally. The returning traffic will bypass rusty
rail, going from the server directly to the router, and thence to the clients.

## Router configuration

Doc patches providing configuration snippets for router models are welcomed :).

In short though: create N GRE tunnel interfaces on the router pointing each at
a rusty rail node. Set the forwarding policy on the router to be stateless
equal cost multipath (or, if you are merely seeking high availability, you
could configure an active/passive tunnel, depending on down host detection
to fail over to the second/third etc rusty rail node.

## Rusty rail configuration

Disable card acceleration for host packets, as at least for the hyper-v netmap
driver, transmit offload doesn't take place when packets are forwarded from
host to nic via the netmap layer. This doesn't impact rusty rail performance,
as all the volume traffic is being handled directly on the NIC ring buffers
rather than via the host networking stack.

## Server configuration

Servers need a local interface with the IP address of the virtual IP(s) they
will be hosting. Secondly they need to accept GRE traffic from any of the rusty
rail nodes and decapsulate those packets.

One easy way to do this is to:
1. Add the virtual IPs as aliases to eth0.
2. Configure GRE tunnels on the server between it and each rusty rail node.

If large numbers or very dynamic rusty rail node details are in use, consider
using ovs to configure openflow rules to perform decap in a wildcard manner.

Make sure any reverse path filtering is disabled (or set to local-only) as
incoming traffic will fail a strict reverse path check.

# Design choices

The basic design is inspired by Maglev. 

## Components

The choice of Rust was part my wanting to spend more time working with one of
the new languages like Rust or Go, and largely a very good fit for the domain:
we need extremely predictable behaviour in the data plane, and languages with
garbage collectors or greenthreads do not supply that.

The choice of networking implementation: raw socket vs Netmap vs DPDK vs SR-IOV
vs IO-Visor XDP is somewhat arbitrary but heavily influenced by my desired to
spend more time working with Rust. DPDK wants to wrap around much more of the
program than is convenient with Rust (good integration is of course
in-principle possible). SR-IOV as the interface for networking would either
require a new abstraction that can deliver networking in testbeds without
SR-IOV hardware - or using Netmap/DPDK etc in a VM with the virtualised card -
not actually changing anything:). XDP is very interesting, and it may well be
worth looking at refactoring the data plane of Rusty rail out of Netmap and
into XDP, with control logic staying in Rust. Netmap provides a lightweight
layer over the actual NIC cards, is compatible with SR-IOV or can also forward
packets to the kernel stack easily, and while its abstractions are awkward in
Rust, a little unsafe() goes a long way.

Over time it may be interesting to introduce a modular system to permit
deployers to make different choices without forking or reimplementing.

## Architecture

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
