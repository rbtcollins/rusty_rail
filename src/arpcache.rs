// Copyright (c) 2016 Robert Collins. Licensed under the Apache-2.0 license.
use std::collections::BTreeMap;
use std::net::Ipv4Addr;
#[cfg(test)]
use std::str::FromStr;
use std::time::{Duration, SystemTime};

use pnet::util::MacAddr;
use pnetlink::packet::netlink::NetlinkConnection;
use pnetlink::packet::route::addr::IpAddr;
use pnetlink::packet::route::link::Link;
use pnetlink::packet::route::link::Links;
use pnetlink::packet::route::neighbour::{Neighbour, Neighbours};

pub struct CacheEntry {
    pub mac: MacAddr,
    pub expires: SystemTime,
}

pub struct Cache {
    pub entries: BTreeMap<Ipv4Addr, CacheEntry>,
    pub link: Link,
    pub netlink: NetlinkConnection,
}

impl Cache {
    pub fn new(link: Link, netlink: NetlinkConnection) -> Cache {
        Cache {
            entries: BTreeMap::new(),
            link: link,
            netlink: netlink,
        }
    }

    pub fn lookup<'a>(&'a mut self, addr: &Ipv4Addr) -> Option<MacAddr> {
        if let Some(result) = self.entries.get(addr).map(|e| e.mac) {
            return Some(result);
        } else {

            let entries = &mut self.entries;
            // TODO Put on negative ttl? Trigger arp by sending a packet? Log
            // status?
            let neighbours =
                self.netlink.iter_neighbours(Some(&self.link)).unwrap().collect::<Vec<_>>();
            for neighbour in neighbours {
                if let Some(mac) = neighbour.get_ll_addr() {
                    if let Some(IpAddr::V4(ipaddr)) = neighbour.get_destination() {
                        //           self.add(&ipaddr, &mac);
                        entries.insert(ipaddr,
                                       CacheEntry {
                                           mac: mac,
                                           expires: SystemTime::now() + Duration::new(30, 0),
                                       });
                    }
                }
            }
            match entries.get(addr).map(|e| e.mac) {
                Some(result) => Some(result),
                None => {
                    None
                    // TODO: trigger a lookup by emitting a packet to the destination address, then
                    // return None.
                }
            }
        }
    }

    pub fn add(&mut self, ip: &Ipv4Addr, mac: &MacAddr) {
        self.entries.insert(*ip,
                            CacheEntry {
                                mac: *mac,
                                expires: SystemTime::now() + Duration::new(30, 0),
                            });
    }

    pub fn expire(&mut self) {
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
    let mut netlink = NetlinkConnection::new();
    let nl_link = netlink.get_link_by_name("eth0").unwrap().unwrap();
    let mut c = Cache::new(nl_link, netlink);
    let target_mac = MacAddr::from_str("aa:aa:aa:aa:aa:aa").unwrap();
    let some_ip = Ipv4Addr::from_str("127.0.0.2").unwrap();
    let past = SystemTime::now() - Duration::new(1, 0);
    c.add(&some_ip, &target_mac);
    c.lookup(&some_ip).unwrap();
    c.expire();
    c.lookup(&some_ip).unwrap();
    c.entries.insert(some_ip,
                     CacheEntry {
                         mac: target_mac,
                         expires: past,
                     });
    c.expire();
    let maybe_ip = c.lookup(&some_ip);
    if let Some(_) = maybe_ip {
        panic!("Failed to expire cached entry.");
    }
}
