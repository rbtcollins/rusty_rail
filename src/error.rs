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
}

impl fmt::Display for BrokenRail {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            BrokenRail::Netmap(ref err) => err.fmt(f),
            BrokenRail::IO(ref err) => err.fmt(f),
            BrokenRail::BadPacket => write!(f, "Couldn't handle packet"),
        }
    }
}

impl error::Error for BrokenRail {
    fn description(&self) -> &str {
        match *self {
            BrokenRail::Netmap(ref err) => err.description(),
            BrokenRail::IO(ref err) => err.description(),
            BrokenRail::BadPacket => "Couldn't handle packet",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            BrokenRail::Netmap(ref err) => Some(err),
            BrokenRail::IO(ref err) => Some(err),
            BrokenRail::BadPacket => None,
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


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
