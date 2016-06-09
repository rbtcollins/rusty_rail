# Rusty rail - a line rate distributed load balancer

Rusty rail is inspired by the [Google Maglev
paper](http://research.google.com/pubs/pub44824.html)

# Current status

Bridges a single interface in full passthrough mode to preserve connections to
host.

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
sudo RR_DEVICE=eth0 cargo run
```

# Configuration

Configuration is via environment variables.

* ``RR_DEVICE`` should be the name of the interface to receive and transmit GRE
  wrapped packets on.

# License

Apache-2.0.
