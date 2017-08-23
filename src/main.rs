// Copyright (c) 2016 Robert Collins. Licensed under the Apache-2.0 license.
extern crate libc;

extern crate netmap;
extern crate pnet;
extern crate pnetlink;

extern crate rusty_rail;


use std::env;
use std::io;
use std::net::{IpAddr, Ipv4Addr};

// use netmap::Direction;
use pnet::datalink::{interfaces, NetworkInterface};

use pnetlink::packet::netlink::NetlinkConnection;
use pnetlink::packet::route::link::Links;

use rusty_rail::arpcache;
use rusty_rail::configuration::Config;
use rusty_rail::error::BrokenRail;
use rusty_rail::{move_packets, TransferStatus};


pub fn poll(pollfds: &mut Vec<libc::pollfd>,
            wire_read: bool,
            host_read: bool)
            -> Result<u8, BrokenRail> {
    // XXX: this needs to be more sophisticated: where we have pending packets for a descriptor, we
    // want POLLOUT; otherwise POLLIN. We don't want POLLIN on a device where the next packet is
    // for an output blocked device.
    //
    // TODO: fix layering violations.
    // 0, 2 we read from.
    // 1 we only write to.
    // Need to have a marker for switching to block-until-we-can-write.
    if wire_read {
        pollfds[0].events = libc::POLLIN;
        pollfds[1].events = 0
    } else {
        pollfds[0].events = 0;
        pollfds[1].events = libc::POLLOUT
    }
    if host_read {
        pollfds[2].events = libc::POLLIN
    } else {
        pollfds[2].events = libc::POLLOUT
    }

    let poll_len = pollfds.len();
    if let Some(first) = pollfds.first_mut() {
        let rv = unsafe { libc::poll(first as *mut libc::pollfd, poll_len as u64, 1000) };
        if rv < 0 {
            Err(BrokenRail::IO(io::Error::last_os_error()))
        } else {
            Ok(rv as u8)
        }
    } else {
        // Nothing to poll, no error.
        return Ok(0);
    }
}


fn pollfd(fd: i32) -> libc::pollfd {
    libc::pollfd {
        fd: fd,
        events: 0,
        revents: 0,
    }
}


fn device_name(device: &String, suffix: &str) -> String {
    device.clone() + suffix
}


fn extract_ipv4(interface: &NetworkInterface) -> Result<Ipv4Addr, BrokenRail> {

    if let Some(ref ips) = interface.ips {
        for ip in ips {
            if let &IpAddr::V4(ipv4) = ip {
                println!("{}", ip);
                return Ok(ipv4);
            }
        }
    }
    return Err(BrokenRail::NoIPV4Address);
}


fn stuff() -> Result<(), BrokenRail> {
    let mut pollfds: Vec<libc::pollfd> = Vec::with_capacity(2);
    let config = try!(Config::new(env::vars()));

    let interface_names_match = {
        |iface: &NetworkInterface| iface.name == config.device
    };
    let interface = interfaces().into_iter()
        .filter(interface_names_match)
        .next()
        .unwrap();
    let interface_ipv4 = try!(extract_ipv4(&interface));

    let mut netlink = NetlinkConnection::new();
    let nl_link = netlink.get_link_by_name(&config.device).unwrap().unwrap();
    let mut arp_cache = arpcache::Cache::new(nl_link, netlink);
    println!("interface {}", interface.mac_address());
    let interface_mac = interface.mac_address();
    // netmap-rs iterators lock the whole NetmapDescriptor, so we open two descriptors for the
    // adapter: one RX only, and on TX only. We open a single bidirectional descriptor for the host
    // side as we have no use case today for looping packets back to the host side.

    let mut nm_in = try!(netmap::NetmapDescriptor::new(&device_name(&config.device, "/R")));
    pollfds.push(pollfd(nm_in.get_fd()));
    println!("wire RX fd {}", pollfds[0].fd);

    let mut nm_out = try!(netmap::NetmapDescriptor::new(&device_name(&config.device, "/T")));
    pollfds.push(pollfd(nm_out.get_fd()));
    println!("wire RX fd {}", pollfds[1].fd);


    let mut nm_host = try!(netmap::NetmapDescriptor::new(&device_name(&config.device, "^")));
    pollfds.push(pollfd(nm_host.get_fd()));
    println!("host fd {}", pollfds[2].fd);

    let mut host_read = true;
    let mut wire_read = true;

    loop {
        if 0 == try!(poll(&mut pollfds, wire_read, host_read)) {
            //       println!("Poll timeout");
            continue;
        }
        host_read = true;
        wire_read = true;
        // A netmap poll error can mean the rings get reset: loop again.
        for pollfd in pollfds.iter() {
            if pollfd.revents & libc::POLLERR == libc::POLLERR {
                continue;
            }
        }
        // println!("Host -> out queue");
        match try!(move_packets(&mut nm_host,
                                &mut nm_out,
                                None,
                                &interface_ipv4,
                                &interface_mac,
                                &config.routes,
                                &mut arp_cache)) {
            TransferStatus::BlockedDestination |
            TransferStatus::BlockedWire => {
                host_read = false;
                wire_read = false
            }
            TransferStatus::Complete => (),
        }
        // println!("Wire -> Host or out queue");
        match try!(move_packets(&mut nm_in,
                                &mut nm_host,
                                Some(&mut nm_out),
                                &interface_ipv4,
                                &interface_mac,
                                &config.routes,
                                &mut arp_cache)) {
            TransferStatus::BlockedDestination => wire_read = false,
            TransferStatus::BlockedWire => host_read = false,
            TransferStatus::Complete => (),
        }
    }
}

fn main() {
    match stuff() {
        Ok(()) => println!("Actual cannibal unreachable code"),
        Err(err) => panic!("{}", err),
    };

}
