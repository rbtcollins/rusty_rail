extern crate netmap;

use netmap::{NetmapSlot, NetmapRing};

pub mod error;


pub fn move_packets(src: &mut netmap::NetmapDescriptor, dst: &mut netmap::NetmapDescriptor) {
    {
        let mut rx_slots = src.rx_iter().flat_map(|rx_ring| rx_ring.iter());
        'rings: for tx_ring in dst.tx_iter() {
            let mut tx_slot_iter = tx_ring.iter_mut();
            'slots: loop {
                match tx_slot_iter.next() {
                    None => break 'slots,
                    Some(tx_slot_buf) => {
                        // println!("Available send slot {:?}", tx_slot_buf.0.get_buf_idx());
                        match rx_slots.next() {
                            None => {
                                // println!("End of RX queue. giving back TX");
                                tx_slot_iter.give_back();
                                break 'rings;
                            }
                            Some(rx_slot_buf) => {
                                // println!("Packet to forward {:?}({})",
                                //         rx_slot_buf.0.get_buf_idx(),
                                //         rx_slot_buf.0.get_len());
                                // XXX: TODO: zero-copy when possible.
                                let tgt_buf =
                                    &mut tx_slot_buf.1[0..rx_slot_buf.0.get_len() as usize];
                                tgt_buf.copy_from_slice(rx_slot_buf.1);
                                tx_slot_buf.0.set_len(rx_slot_buf.0.get_len());
                            }
                        }
                    }
                }
            }
        }
    };
    for ring in src.rx_iter() {
        ring.head_from_cur();
    }
    for ring in dst.tx_iter() {
        ring.head_from_cur();
    }
}


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
