// Copyright (c) 2016 Robert Collins. Licensed under the Apache-2.0 license.
use std::collections::BTreeMap;
use std::net::Ipv4Addr;
use std::str::FromStr;

use pnet::util::MacAddr;

use super::error;

pub struct Config {
    pub device: String,
    pub target_ip: Ipv4Addr,
}

impl Config {
    pub fn new<I>(vars: I) -> Result<Config, error::BrokenRail>
        where I: Iterator<Item = (String, String)>
    {
        let vars: BTreeMap<String, String> = vars.collect();
        let target_ip = Ipv4Addr::from_str(&vars.get("RR_TARGET_IP").unwrap()).unwrap();
        Ok(Config {
            device: vars.get("RR_DEVICE").unwrap().clone(),
            target_ip: target_ip,
        })
    }
}

#[test]
fn set_variables() {
    let vars = [("RR_DEVICE".to_string(), "wlan0".to_string()),
                ("RR_TARGET_MAC".to_string(), "ab:cd:ef:01:23:45".to_string()),
                ("RR_TARGET_IP".to_string(), "192.0.2.1".to_string())];
    let config = Config::new(vars.iter().cloned()).unwrap();
    assert_eq!(config.device, "wlan0");
    assert_eq!(config.target_ip, Ipv4Addr::from_str("192.0.2.1").unwrap());
}

#[test]
#[should_panic]
#[allow(unused_must_use)]
fn no_device_error() {
    let vars = [("RR_TARGET_MAC".to_string(), "ab:cd:ef:01:23:45".to_string()),
                ("RR_TARGET_IP".to_string(), "192.0.2.1".to_string())];
    Config::new(vars.iter().cloned());
}

#[test]
#[should_panic]
#[allow(unused_must_use)]
fn no_mac_error() {
    let vars = [("RR_DEVICE".to_string(), "wlan0".to_string()),
                ("RR_TARGET_IP".to_string(), "192.0.2.1".to_string())];
    Config::new(vars.iter().cloned());
}

#[test]
#[should_panic]
#[allow(unused_must_use)]
fn no_ip_error() {
    let vars = [("RR_TARGET_MAC".to_string(), "ab:cd:ef:01:23:45".to_string()),
                ("RR_DEVICE".to_string(), "wlan0".to_string())];
    Config::new(vars.iter().cloned());
}
