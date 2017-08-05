// Copyright (c) 2016 Robert Collins. Licensed under the Apache-2.0 license.
extern crate netmap;
extern crate pnet;
extern crate pnetlink;

use std::hash::{Hash, SipHasher, Hasher};
use std::net::Ipv4Addr;

use netmap::{NetmapSlot, NetmapRing};
use pnet::packet::ethernet::{EthernetPacket, MutableEthernetPacket};
use pnet::packet::ethernet::EtherTypes::Ipv4;
use pnet::packet::ipv4::{Ipv4Packet, MutableIpv4Packet};
use pnet::packet::{MutablePacket, Packet};
use pnet::packet::ip::IpNextHeaderProtocols::Gre;
use pnet::packet::gre;
use pnet::util::MacAddr;

pub mod arpcache;
pub mod configuration;
pub mod error;

enum Direction {
    Destination,
    Drop,
    Wire,
}

pub enum TransferStatus {
    BlockedDestination,
    BlockedWire,
    Complete,
}


fn hash_ipv4_packet(packet: &Ipv4Packet) -> u64 {
    let mut s = SipHasher::new();
    packet.get_source().hash(&mut s);
    packet.get_destination().hash(&mut s);
    packet.get_next_level_protocol().hash(&mut s);
    // Should we add ports in here? Maglev does, but no reason is given.
    s.finish()
}

/// Move a packet from one ring to another.
///
/// rx_slot_buf is the receipt slot to move from
/// tx_slot_buf is the transmission slot to move it into
fn move_packet(rx_slot_buf: (&mut netmap::RxSlot, &mut [u8]),
               tx_slot_buf: (&mut netmap::TxSlot, &mut [u8]))
               -> Result<(), error::BrokenRail> {
    // XXX: TODO: zero-copy when possible.
    let tgt_buf = &mut tx_slot_buf.1[0..rx_slot_buf.0.get_len() as usize];
    tgt_buf.copy_from_slice(rx_slot_buf.1);
    tx_slot_buf.0.set_len(rx_slot_buf.0.get_len());
    Ok(())
}

// Debug code
//
// #[cfg(debug_assertions)]
// fn debug_gre() {
// println!("GRE protocol {:#06X} flags=checksum_present {:?}, routing {:?}, key {:?}, sequence \
// {:?}",
// gre.get_protocol_type(),
// gre.get_checksum_present(),
// gre.get_routing_present(),
// gre.get_key_present(),
// gre.get_sequence_present());
// }


type RxSlotBuf<'a> = (&'a mut netmap::RxSlot, &'a mut [u8]);

#[allow(non_upper_case_globals)]
/// Determine the interface (and when appropriate new targets) for a single packet.
///
/// rx_slot_buf is a packet that has been received.
fn examine_one<'a>(rx_slot_buf: RxSlotBuf) -> Result<Direction, error::BrokenRail> {
    let packet = match EthernetPacket::new(rx_slot_buf.1) {
        Some(packet) => packet,
        None => return Err(error::BrokenRail::BadPacket),
    };
    match packet.get_ethertype() {
        Ipv4 => {
            if let Some(ip) = Ipv4Packet::new(packet.payload()) {
                match ip.get_next_level_protocol() {
                    Gre => {
                        if let Some(gre) = gre::GrePacket::new(ip.payload()) {
                            match gre.get_protocol_type() {
                                0x0800 => {
                                    if let Some(inner_ip) = Ipv4Packet::new(gre.payload()) {
                                        println!("Inner IP {:?} {:?} {:?}",
                                                 inner_ip.get_source(),
                                                 inner_ip.get_destination(),
                                                 hash_ipv4_packet(&inner_ip));
                                    }
                                    // try!(move_packet(rx_slot_buf, tx_slot_buf));
                                    return Ok(Direction::Wire);
                                }
                                // Drop all other gre packets as noise
                                _ => return Ok(Direction::Drop),
                            }

                        } else {
                            println!("Failed to process GRE packet");
                            return Ok(Direction::Drop);
                        }
                        // Drop (in future forward
                        // println!("packet {:?}",
                        //         ip.get_next_level_protocol())
                        return Ok(Direction::Drop);
                    }
                    // Forward non-GRE
                    _ => return Ok(Direction::Destination),
                    // try!(move_packet(rx_slot_buf, tx_slot_buf)),
                }
            } else {
                return Err(error::BrokenRail::BadPacket);
            };
            // println!("packet {:?}", packet.get_ethertype())
        }
        // Forward non-IPV4 packets - ARP etc
        _ => return Ok(Direction::Destination),
        // try!(move_packet(rx_slot_buf, tx_slot_buf)),
    }
}


#[allow(non_upper_case_globals)]
pub fn move_packets(src: &mut netmap::NetmapDescriptor,
                    dst: &mut netmap::NetmapDescriptor,
                    mut maybe_wire: Option<&mut netmap::NetmapDescriptor>,
                    interface_ipv4: &Ipv4Addr,
                    interface_mac: &MacAddr,
                    target_ipv4s: &Vec<Ipv4Addr>,
                    arp_cache: &mut arpcache::Cache)
                    -> Result<TransferStatus, error::BrokenRail> {
    {
        // We need up to three iterators:
        // RX from src
        // TX from dst
        // TX from wire, if src is src is wire.
        //
        // The inner loop will advance the relevant TX iterator as needed, and return when a
        // received packet can't be processed (because of no outbound buffer space).
        //
        // In future, we could allocate additional buffers from netmap and switch received buffers
        // out of the ring to permit more reads to take place while we process packets - whether
        // thats waiting for tx to free up or using a worker thread pool
        let target_ipv4 = &target_ipv4s[0];
        let mut dst_slots = dst.tx_iter().flat_map(|tx_ring| tx_ring.iter_mut());
        let mut maybe_wire_slots = match maybe_wire {
            None => None,
            Some(ref mut wire) => Some(wire.tx_iter().flat_map(|tx_ring| tx_ring.iter_mut())),
        };
        'rx: for rx_ring in src.rx_iter() {
            let mut rx_slot_iter = rx_ring.iter();
            'rx_slot: loop {
                match rx_slot_iter.next() {
                    None => break 'rx,
                    Some((rx_slot, buf)) => {
                        // We have a received packet.
                        let direction = try!(examine_one((rx_slot, buf)));
                        let maybe_tx_slot_buf = match direction {
                            Direction::Destination => dst_slots.next(),
                            Direction::Drop => continue 'rx_slot,
                            Direction::Wire => {
                                match maybe_wire_slots {
                                    None => dst_slots.next(),
                                    Some(ref mut wire_slots) => wire_slots.next(),
                                }
                            }
                        };
                        if let Direction::Wire = direction {
                            let mut packet = match MutableEthernetPacket::new(buf) {
                                Some(packet) => packet,
                                None => return Err(error::BrokenRail::BadPacket),
                            };
                            // We received it, now we're sending it.
                            {
                                let t = packet.get_destination();
                                packet.set_source(t);
                            }
                            let ip_pkt_dest;
                            if let Some(ref mut ip) =
                                   MutableIpv4Packet::new(packet.payload_mut()) {
                                {
                                    let t = ip.get_destination();
                                    ip.set_source(t);
                                }
                                ip_pkt_dest = *target_ipv4;
                                ip.set_destination(ip_pkt_dest);
                                // let tmp_mac = packet.get_source();
                                // packet.set_source(packet.get_destination());
                                // packet.set_destination(tmp_mac);
                                // TODO HERE:
                                // - update outer source MAC
                                // - update outer source IP
                                // - update outer dest MAC
                                // - update outer dest IP
                                // profit
                            } else {
                                // Not a valid IPv4 packet - discard it: 
                                continue 'rx_slot;
                                // return Err(error::BrokenRail::BadPacket);
                            }
                            // ////// move ipv4 lookup to outside; set ipv4, then
                            //do arp cache lookup - helper fn time?
                            match arp_cache.lookup(&ip_pkt_dest) {
                                Some(target_mac) => {
                                    packet.set_destination(target_mac);
                                }
                                None => {
                                    // println!("Dropping {:?}", packet);
                                    // Drop the packet: without a spare buffer to put the packet
                                    // in, the recieve ring will rapidly block.
                                    continue 'rx_slot;
                                }
                            }
                        };
                        if let Some(tx_slot_buf) = maybe_tx_slot_buf {
                            try!(move_packet((rx_slot, buf), tx_slot_buf));
                        } else {
                            // Couldn't get a tx slot, break out to the event loop.
                            // We should perhaps instead discard the packet: if we can't transmit
                            // do we really want to stall entirely?
                            rx_slot_iter.give_back();
                            return Ok(match direction {
                                Direction::Destination => TransferStatus::BlockedDestination,
                                Direction::Drop => panic!("Unreachable"),
                                Direction::Wire => TransferStatus::BlockedWire,
                            });
                        }
                    }
                }
            }
        }
    };
    // These need a comment : the use of early return may interact. Someone should think of these
    // things!
    for ring in src.rx_iter() {
        ring.head_from_cur();
    }
    for ring in dst.tx_iter() {
        ring.head_from_cur();
    }
    if let Some(wire) = maybe_wire {
        for ring in wire.tx_iter() {
            ring.head_from_cur();
        }
    }
    Ok(TransferStatus::Complete)
}


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
