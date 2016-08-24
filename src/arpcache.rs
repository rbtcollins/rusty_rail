// Copyright (c) 2016 Robert Collins. Licensed under the Apache-2.0 license.
use std::collections::BTreeMap;
use std::net::Ipv4Addr;
#[cfg(test)]
use std::str::FromStr;
use std::time::{Duration, SystemTime};

use pnet::util::MacAddr;

pub struct CacheEntry {
    pub mac: MacAddr,
    pub expires: SystemTime,
}

pub struct Cache {
    pub entries: BTreeMap<Ipv4Addr, CacheEntry>,
}

impl Cache {

    pub fn new() -> Cache { 
        Cache {entries: BTreeMap::new()}
    }

    pub fn lookup<'a>(&'a mut self, addr: &Ipv4Addr) -> Option<&'a MacAddr> {
        self.entries.get(addr).map(|e| &e.mac)
    }

    pub fn add(& mut self, ip: &Ipv4Addr, mac: &MacAddr) {
        self.entries.insert(*ip, CacheEntry {mac: *mac, expires:SystemTime::now()+Duration::new(30,0)});
    }

    pub fn expire(& mut self) {
        let now = SystemTime::now();
        let mut expired: Vec<Ipv4Addr> = Vec::new();
        for (ip, entry) in self.entries.iter_mut() {
            if entry.expires < now {
                println!("expiring");
                expired.push(*ip);
            }
        }
        for ip in expired {
            self.entries.remove(&ip);
        }
    }
}

#[test]
fn add_and_lookup_expire() {
    let mut c = Cache::new();
    let target_mac = MacAddr::from_str("aa:aa:aa:aa:aa:aa").unwrap();
    let some_ip = Ipv4Addr::from_str("127.0.0.1").unwrap();
    let past = SystemTime::now() - Duration::new(1,0);
    c.add(&some_ip, &target_mac);
    c.lookup(&some_ip).unwrap();
    c.expire();
    c.lookup(&some_ip).unwrap();
    c.entries.insert(some_ip, CacheEntry {mac: target_mac, expires: past});
    c.expire();
    let maybe_ip = c.lookup(&some_ip);
    if let Some(_) = maybe_ip {
        panic!("Failed to expire cached entry.");
    }
}
