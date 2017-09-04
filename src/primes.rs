// Copyright (c) 2017 Robert Collins. Licensed under the Apache-2.0 license.
//
// Used in implementing the maglev consistent hash.

#[test]
fn examples() {
    assert_eq!(primes(0), vec![]);
    assert_eq!(primes(1), vec![]);
    assert_eq!(primes(2), vec![2]);
    assert_eq!(primes(3), vec![2, 3]);
    assert_eq!(primes(4), vec![2, 3]);
    assert_eq!(primes(5), vec![2, 3, 5]);
}

/// Returns all primes less than or equal to limit in a vector
pub fn primes(limit: usize) -> Vec<u32> {
    let sqrt = (limit as f64).sqrt().ceil() as usize;
    let mut res: Vec<u32> = Vec::with_capacity(sqrt);
    let mut sieve = vec![false; limit+1];
    if limit < 2 {
        return res;
    }
    for candidate in 2..limit + 1 {
        if sieve[candidate] {
            continue;
        }
        res.push(candidate as u32);
        let mut composite = 2 * candidate;
        while composite <= limit {
            sieve[composite] = true;
            composite += candidate
        }
    }
    res
}
