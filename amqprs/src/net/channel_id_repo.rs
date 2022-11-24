use amqp_serde::types::{AmqpChannelId, ShortUint};

use crate::frame::DEFAULT_CONN_CHANNEL;

const INITIAL_BIT_MASK: u8 = 0b1000_0000;
pub(crate) struct ChannelIdRepository {
    /// Each bit represent two states: 1: occupied, 0: free.
    /// Real id is calculated by byte postion in Vec + bit postion in byte.
    id_state: Vec<u8>,
}
impl ChannelIdRepository {
    pub fn new(channel_max: ShortUint) -> Self {
        let len = 1 + (channel_max as usize - 1) / 8;

        Self {
            id_state: vec![0; len],
        }
    }

    fn is_free(&mut self, pos: usize, mask: u8) -> bool {
        (mask & self.id_state[pos]) == 0
    }

    fn set_occupied(&mut self, pos: usize, mask: u8) {
        self.id_state[pos] |= mask;
    }

    fn set_free(&mut self, pos: usize, mask: u8) {
        self.id_state[pos] &= !mask;
    }

    fn get_pos_mask(&self, id: AmqpChannelId) -> (usize, u8) {
        let pos = (id as usize - 1) / 8;
        let mask = INITIAL_BIT_MASK >> ((id - 1) % 8);
        (pos, mask)
    }

    pub fn allocate(&mut self) -> AmqpChannelId {
        let pos = self
            .id_state
            .iter()
            .position(|&v| v != 0b1111_1111)
            .expect("id allocation never fail");
        for i in 0..8 {
            let mask = INITIAL_BIT_MASK >> i;
            if self.is_free(pos, mask) {
                // mark it as occupied
                self.set_occupied(pos, mask);
                // calculate the real id
                let channel_id = pos as AmqpChannelId * 8 + i + 1;
                return channel_id;
            }
        }
        unreachable!("id allocation should always return");
    }
    /// true: OK, false: already released
    pub fn release(&mut self, id: AmqpChannelId) -> bool {
        assert_ne!(0, id, "connection's default channel 0 cannot be released");

        let (pos, mask) = self.get_pos_mask(id);
        if self.is_free(pos, mask) {
            // already released
            false
        } else {
            self.set_free(pos, mask);
            true
        }
    }

    /// true: OK, false: already reserved
    pub fn reserve(&mut self, id: AmqpChannelId) -> bool {
        assert_ne!(0, id, "connection's default channel 0 cannot be reserved");
        let (pos, mask) = self.get_pos_mask(id);

        if !self.is_free(pos, mask) {
            // already occupied
            false
        } else {
            self.set_occupied(pos, mask);
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::ChannelIdRepository;

    #[test]
    fn test_id_allocate_and_release() {
        let channel_max = 2047;
        let mut id_repo = ChannelIdRepository::new(channel_max);

        let mut ids = HashSet::new();
        // allocate to max
        for _ in 0..channel_max {
            let id = id_repo.allocate();
            // id should be unique
            assert_eq!(true, ids.insert(id));
        }
        // free all
        for id in ids {
            assert_eq!(true, id_repo.release(id));
        }
        //can allocte to max again
        let mut ids = HashSet::new();

        for _ in 0..channel_max {
            let id = id_repo.allocate();
            // id should be unique
            assert_eq!(true, ids.insert(id));
        }
    }

    #[test]
    fn test_id_reserve_and_release() {
        let channel_max = 2047;
        let mut id_repo = ChannelIdRepository::new(channel_max);

        let mut ids = vec![];
        // reserver all id: from '1' to max
        for i in 1..channel_max + 1 {
            assert_eq!(true, id_repo.reserve(i));
            ids.push(i);
        }
        // free all
        for id in ids {
            assert_eq!(true, id_repo.release(id));
        }
        // can allocte to max again
        for _ in 0..channel_max {
            id_repo.allocate();
        }
    }

    #[test]
    fn test_cannot_reserve_occupied_id() {
        let channel_max = 2047;
        let mut id_repo = ChannelIdRepository::new(channel_max);

        let mut ids = HashSet::new();
        // allocate to max
        for _ in 0..channel_max {
            let id = id_repo.allocate();
            // id should be unique
            assert_eq!(true, ids.insert(id));
        }
        // failed to reserve
        for id in ids {
            assert_eq!(false, id_repo.reserve(id));         
        }
    }
}
