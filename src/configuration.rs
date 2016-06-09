// Copyright (c) 2016 Robert Collins. Licensed under the Apache-2.0 license.
use std::collections::BTreeMap;

use super::error;

pub struct Config {
    pub device: String,
}

impl Config {
    pub fn new<I>(vars: I) -> Result<Config, error::BrokenRail>
        where I: Iterator<Item = (String, String)>
    {
        let vars: BTreeMap<String, String> = vars.collect();
        Ok(Config { device: vars.get("RR_DEVICE").unwrap().clone() })
    }
}

#[test]
#[should_panic]
#[allow(unused_must_use)]
fn no_device_error() {
    let vars: [(String, String); 0] = [];
    Config::new(vars.iter().cloned());
}

#[test]
fn set_device() {
    let vars = [("RR_DEVICE".to_string(), "wlan0".to_string())];
    let config = Config::new(vars.iter().cloned()).unwrap();
    assert_eq!(config.device, "wlan0");
}
