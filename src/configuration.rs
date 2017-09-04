// Copyright (c) 2016 Robert Collins. Licensed under the Apache-2.0 license.
use std::collections::BTreeMap;
use std::net::Ipv4Addr;
use std::str::FromStr;

use super::error;
use super::consistenthash::{Backend, ConsistentHash};

pub struct Config {
    pub device: String,
    pub routes: ConsistentHash,
    pub target_ips: Vec<Ipv4Addr>,
}

impl Config {
    pub fn new<I>(vars: I) -> Result<Config, error::BrokenRail>
        where I: Iterator<Item = (String, String)>
    {
        let vars: BTreeMap<String, String> = vars.collect();
        let ipstring = &vars.get("RR_TARGET_IPS").unwrap();
        let target_ips: Vec<Ipv4Addr> =
            ipstring.split(";").map(|i| Ipv4Addr::from_str(&i).unwrap()).collect();
        let mut hash = ConsistentHash::new();
        for target_name in ipstring.split(";") {
            let ip = Ipv4Addr::from_str(&target_name).unwrap();
            let backend = Backend::new(&target_name, ip);
            hash.backends.push(backend);
        }
        hash.populate();
        Ok(Config {
            device: vars.get("RR_DEVICE").unwrap().clone(),
            routes: hash,
            target_ips: target_ips,
        })
    }
}

#[test]
fn set_variables() {
    let vars = [("RR_DEVICE".to_string(), "wlan0".to_string()),
                ("RR_TARGET_IPS".to_string(), "192.0.2.1".to_string())];
    let config = Config::new(vars.iter().cloned()).unwrap();
    assert_eq!(config.device, "wlan0");
    assert_eq!(config.target_ips[0],
               Ipv4Addr::from_str("192.0.2.1").unwrap());
}

#[test]
fn multiple_ips() {
    let vars = [("RR_DEVICE".to_string(), "wlan0".to_string()),
                ("RR_TARGET_IPS".to_string(), "192.0.2.1;192.0.2.2".to_string())];
    let config = Config::new(vars.iter().cloned()).unwrap();
    assert_eq!(config.device, "wlan0");
    assert_eq!(config.target_ips,
               vec![Ipv4Addr::from_str("192.0.2.1").unwrap(),
                    Ipv4Addr::from_str("192.0.2.2").unwrap()]);
}

#[test]
#[should_panic]
#[allow(unused_must_use)]
fn no_device_error() {
    let vars = [("RR_TARGET_IPS".to_string(), "192.0.2.1".to_string())];
    Config::new(vars.iter().cloned());
}

#[test]
#[should_panic]
#[allow(unused_must_use)]
fn no_ip_error() {
    let vars = [("RR_DEVICE".to_string(), "wlan0".to_string())];
    Config::new(vars.iter().cloned());
}

#[test]
#[should_panic]
#[allow(unused_must_use)]
fn zero_length_ips_error() {
    let vars = [("RR_DEVICE".to_string(), "wlan0".to_string()),
                ("RR_TARGET_IPS".to_string(), "".to_string())];
    Config::new(vars.iter().cloned());
}
