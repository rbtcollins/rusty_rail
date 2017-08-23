// Copyright (c) 2017 Robert Collins. Licensed under the Apache-2.0 license.
//
// Consistent hashing for selecting backends.

use std::hash::{Hash, Hasher};
use std::net::Ipv4Addr;

use siphasher::sip::SipHasher;

use super::primes;

pub struct Backend {
    pub name: String,
    /// Should this backend receive new traffic.
    pub live: bool,
    pub target: Ipv4Addr, /* | Ipv6Addr, weight  */
    pub permutation: Vec<u32>,
}

impl Backend {
    pub fn new(name: &str, target: Ipv4Addr) -> Backend {
        Backend {
            name: name.to_string(),
            live: true,
            target: target,
            permutation: vec![],
        }
    }
}

pub struct ConsistentHash {
    // vector of known backends. References to this are held by lookup using their offset: never
    // shrink except by marking the tail nodes not alive, populating the lookup table, then finally
    // popping.
    pub backends: Vec<Backend>,
    pub lookup: Vec<u32>,
}


impl ConsistentHash {
    pub fn new() -> ConsistentHash {
        ConsistentHash {
            backends: vec![],
            lookup: vec![],
        }
    }

    /// Populate the lookup table based on the current backend settings.
    ///
    /// ```
    /// use std::net::Ipv4Addr;
    /// use std::str::FromStr;
    ///
    /// use rusty_rail::consistenthash;
    ///
    /// let mut c = consistenthash::ConsistentHash::new();
    /// let tgt = Ipv4Addr::from_str("1.2.3.4").unwrap();
    /// c.backends.push(consistenthash::Backend::new("server-1", tgt));
    /// c.backends.push(consistenthash::Backend::new("server-2", tgt));
    /// c.backends.push(consistenthash::Backend::new("server-3", tgt));
    /// c.backends.push(consistenthash::Backend::new("server-4", tgt));
    /// c.backends[3].live = false;
    /// c.populate();
    /// assert_eq!(c.lookup,
    /// vec!
    /// [2, 1, 1, 0, 1, 2, 0, 1, 2, 1, 2, 0, 2, 2, 0, 2, 1, 0, 2, 1, 2, 2, 0, 2, 0, 0, 1, 0, 1, 2,
    /// 0, 1, 0, 0, 2, 0, 2, 2, 1, 2, 1, 0, 2, 1, 2, 2, 0, 2, 0, 0, 2, 0, 1, 0, 0, 1, 0, 0, 2, 0, 0,
    /// 1, 1, 2, 1, 0, 2, 1, 0, 1, 0, 2, 0, 0, 2, 0, 0, 0, 0, 1, 0, 0, 1, 1, 0, 1, 1, 2, 1, 0, 1, 1,
    /// 0, 1, 0, 2, 0, 0, 1, 0, 0, 2, 0, 0, 0, 1, 1, 1, 2, 1, 1, 2, 1, 2, 1, 1, 2, 1, 2, 0, 2, 2, 1,
    /// 2, 2, 2, 2, 1, 2, 2, 1, 1, 2, 1, 1, 0, 1, 2, 1, 1, 2, 1, 2, 0, 2, 2, 0, 2, 0, 2, 2, 1, 2, 2,
    /// 0, 2, 1, 2, 1, 0, 1, 2, 0, 1, 0, 1, 2, 0, 1, 2, 0, 2, 0, 2, 2, 1, 2, 2, 0, 2, 1, 0, 2, 0, 2,
    /// 0, 0, 1, 0, 0, 2, 0, 1, 0, 0, 2, 1, 0, 2, 1, 2, 1, 0, 2, 1, 0, 2, 0, 2, 0, 0, 1, 0, 0, 0, 0,
    /// 1, 1, 1, 2, 1, 0, 1, 1, 0, 1, 0, 2, 1, 0, 1, 0, 0, 0, 0, 2, 0, 0, 0, 0, 1, 1, 1, 1, 1, 0, 1,
    /// 1, 0, 1, 0, 1, 1, 2, 1, 2, 2, 2, 2, 1, 2, 2, 1, 2, 1, 1, 1, 0, 1, 2, 1, 1, 2, 1, 2, 0, 1, 2,
    /// 1, 2, 0, 2, 2, 0, 2, 2, 2, 2, 1, 2, 2, 0, 1]);
    /// ```
    pub fn populate(&mut self) {
        // This is 'approximately' 100x the number of backends; could look for the next higher
        // prime in future to ensure that.
        let lookup_size = self.backends.iter().filter(|b| b.live).count() * 100;
        let p = primes::primes(lookup_size);
        let lookup_size = p[p.len() - 1] as u32;
        for backend in &mut self.backends {
            if backend.permutation.len() != lookup_size as usize {
                backend.permutation = permute_backend(&backend.name, lookup_size)
            }
        }
        let mut next = vec![0; self.backends.len()];
        self.lookup = vec![u32::max_value();lookup_size as usize];
        let mut allocated = 0;
        loop {
            for (i, backend) in self.backends.iter().enumerate() {
                if !backend.live {
                    continue;
                }
                let mut candidate = backend.permutation[next[i]];
                while self.lookup[candidate as usize] != u32::max_value() {
                    // Find next unallocated position from backend.
                    next[i] += 1;
                    candidate = backend.permutation[next[i]];
                }
                self.lookup[candidate as usize] = i as u32;
                next[i] += 1;
                allocated += 1;
                if allocated == lookup_size {
                    return;
                }
            }
        }
    }
}

/// Generate permutations for a given offset, skip, pool
///
/// ```
/// use rusty_rail::consistenthash::permutations;
///
/// assert_eq!(permutations(3, 4, 7), vec![3, 0, 4, 1, 5, 2, 6]);
/// assert_eq!(permutations(0, 2, 7), vec![0, 2, 4, 6, 1, 3, 5]);
/// assert_eq!(permutations(3, 1, 7), vec![3, 4, 5, 6, 0, 1, 2]);
/// ```
pub fn permutations(offset: u32, skip: u32, pool_size: u32) -> Vec<u32> {
    let mut res: Vec<u32> = Vec::with_capacity(pool_size as usize);
    for pos in 0..pool_size {
        res.push((offset + pos * skip) % pool_size)
    }
    res
}

/// Generate permutations for a given name and pool size.
///
/// ```
/// use rusty_rail::consistenthash::permute_backend;
///
/// assert_eq!(permute_backend("fred", 7), vec![1, 0, 6, 5, 4, 3, 2]);
/// assert_eq!(permute_backend("ralph", 7), vec![3, 2, 1, 0, 6, 5, 4]);
/// assert_eq!(permute_backend("larry", 7), vec![4, 0, 3, 6, 2, 5, 1]);
/// ```
pub fn permute_backend(name: &str, pool_size: u32) -> Vec<u32> {
    // May be faster to generate just-in-time as an iterator: profile eventually.
    let mut s = SipHasher::new();
    name.hash(&mut s);
    let offset = (s.finish() % pool_size as u64) as u32;
    "differenthash".hash(&mut s);
    let skip = (s.finish() % (pool_size as u64 - 1) + 1) as u32;
    permutations(offset, skip, pool_size)
}
