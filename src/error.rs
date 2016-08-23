// Copyright (c) 2016 Robert Collins. Licensed under the Apache-2.0 license.
extern crate netmap;

use std::error;
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum BrokenRail {
    Netmap(netmap::NetmapError),
    IO(io::Error),
    BadPacket,
    NoIPV4Address,
}

impl fmt::Display for BrokenRail {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            BrokenRail::Netmap(ref err) => err.fmt(f),
            BrokenRail::IO(ref err) => err.fmt(f),
            BrokenRail::BadPacket => write!(f, "Couldn't handle packet"),
            BrokenRail::NoIPV4Address => write!(f, "No IPV4 address on interface"),
        }
    }
}

impl error::Error for BrokenRail {
    fn description(&self) -> &str {
        match *self {
            BrokenRail::Netmap(ref err) => err.description(),
            BrokenRail::IO(ref err) => err.description(),
            BrokenRail::BadPacket => "Couldn't handle packet",
            BrokenRail::NoIPV4Address => "No IPV4 address on interface",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            BrokenRail::Netmap(ref err) => Some(err),
            BrokenRail::IO(ref err) => Some(err),
            BrokenRail::BadPacket => None,
            BrokenRail::NoIPV4Address => None,
        }
    }
}

impl From<netmap::NetmapError> for BrokenRail {
    fn from(err: netmap::NetmapError) -> BrokenRail {
        BrokenRail::Netmap(err)
    }
}


impl From<io::Error> for BrokenRail {
    fn from(err: io::Error) -> BrokenRail {
        BrokenRail::IO(err)
    }
}
