// Copyright (c) 2017 Robert Collins. Licensed under the Apache-2.0 license.
//
// Consistent hashing for selecting backends.

use std::hash::{Hash, Hasher};

use siphasher::sip::{SipHasher};

use super::primes;

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
/// assert_eq!(permute_backend("fred".to_string(), 7), vec![1, 0, 6, 5, 4, 3, 2]);
/// assert_eq!(permute_backend("ralph".to_string(), 7), vec![3, 2, 1, 0, 6, 5, 4]);
/// assert_eq!(permute_backend("larry".to_string(), 7), vec![4, 0, 3, 6, 2, 5, 1]);
/// ```
pub fn permute_backend(name: String, pool_size: u32) -> Vec<u32> {
    let mut s = SipHasher::new();
    name.hash(&mut s);
    let offset = (s.finish() % pool_size as u64) as u32;
    "differenthash".hash(&mut s);
    let skip = (s.finish() % (pool_size as u64 - 1) + 1) as u32;
    permutations(offset, skip, pool_size)
}
