// Copyright (c) 2016 Robert Collins. Licensed under the Apache-2.0 license.
extern crate libc;

extern crate netmap;

extern crate rusty_rail;


use std::io;

use netmap::Direction;

use rusty_rail::error::BrokenRail;
use rusty_rail::move_packets;


pub fn poll(pollfds: &mut Vec<libc::pollfd>) -> Result<u8, BrokenRail> {
    // XXX: this needs to be more sophisticated: where we have pending packets for a descriptor, we
    // want POLLOUT; otherwise POLLIN.
    let dir = Direction::Input;
    // Reset the events we're waiting for.
    for pollfd in pollfds.iter_mut() {
        pollfd.events = match dir {
            Direction::Input => libc::POLLIN,
            Direction::Output => libc::POLLOUT,
            Direction::InputOutput => libc::POLLIN | libc::POLLOUT,
        };
        pollfd.revents = 0;
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



fn stuff() -> Result<(), BrokenRail> {
    let mut pollfds: Vec<libc::pollfd> = Vec::with_capacity(2);

    let mut nm_in_host = try!(netmap::NetmapDescriptor::new("eth0^"));
    pollfds.push(pollfd(nm_in_host.get_fd()));
    println!("host fd {}", pollfds[0].fd);

    let mut nm_in = try!(netmap::NetmapDescriptor::new("eth0"));
    pollfds.push(pollfd(nm_in.get_fd()));
    println!("wire fd {}", pollfds[1].fd);

    loop {
        if 0 == try!(poll(&mut pollfds)) {
            //       println!("Poll timeout");
            continue;
        }
        // A netmap poll error can mean the rings get reset: loop again.
        for pollfd in pollfds.iter() {
            if pollfd.revents & libc::POLLERR == libc::POLLERR {
                continue;
            }
        }
        // println!("Host -> Wire");
        move_packets(&mut nm_in_host, &mut nm_in);
        // println!("Wire -> Host");
        move_packets(&mut nm_in, &mut nm_in_host);
    }
}

fn main() {
    match stuff() {
        Ok(()) => println!("Actual cannibal unreachable code"),
        Err(err) => panic!("{}", err),
    };

}
