use std::{collections::HashMap, hash::Hash};

use super::HeapError;

#[derive(Debug)]
pub struct MinHeapMap<K, T> {
    data: Vec<T>,
    lookup: HashMap<K, usize>,
    length: usize,
}

pub trait MinMapHeapable<K: Hash + PartialEq>:
    PartialEq + PartialOrd + Clone + std::fmt::Debug
{
    fn lookup_key(&self) -> K;
}

impl<K: Hash + PartialEq + Eq, T: MinMapHeapable<K>> From<Vec<T>> for MinHeapMap<K, T> {
    fn from(value: Vec<T>) -> Self {
        let mut new = Self::new();
        for v in value {
            new.insert(v);
        }
        assert_eq!(new.data.len(), new.lookup.len());
        assert_eq!(new.data.len(), new.length);

        new
    }
}

impl<K: Hash + PartialEq + Eq, T: MinMapHeapable<K>> MinHeapMap<K, T> {
    pub fn new() -> Self {
        Self {
            data: vec![],
            lookup: HashMap::new(),
            length: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.length
    }

    pub fn insert(&mut self, val: T) {
        self.lookup.insert(val.lookup_key(), self.length + 1);
        self.data.push(val);
        self.heapify_up(self.length);
        self.length += 1;
    }

    pub fn lookup(&self, key: K) -> Option<&T> {
        if let Some(idx) = self.lookup.get(&key) {
            return Some(
                self.data
                    .get(*idx)
                    .expect("key exists, but not present in data"),
            );
        }
        None
    }

    pub fn lookup_and_mutate(&mut self, key: K, f: impl FnOnce(&mut T)) -> Result<(), HeapError> {
        if let Some(idx) = self.lookup.get(&key) {
            if let Some(data) = self.data.get_mut(*idx) {
                f(data);
                self.heapify_up(*idx);
                return Ok(());
            }
        }
        Err(HeapError::LookupReturnedNone)
    }

    pub fn pop(&mut self) -> Result<T, HeapError> {
        if self.length == 0 {
            return Err(HeapError::LengthIsZero);
        }
        let out = self.data.remove(0);
        self.lookup.remove(&out.lookup_key()).unwrap();
        self.length -= 1;

        if self.length == 0 {
            self.data = vec![];
            self.length = 0;
            return Ok(out);
        }
        self.heapify_down(0);
        Ok(out)
    }

    fn swap(&mut self, one: usize, other: usize) {
        let one_val = self.data[one].clone();
        let other_val = self.data[other].clone();

        let one_lookup_key = one_val.lookup_key();
        self.lookup.insert(one_lookup_key, other);
        self.data[other] = one_val;

        let other_lookup_key = other_val.lookup_key();
        self.lookup.insert(other_lookup_key, one);
        self.data[one] = other_val;
    }

    fn heapify_down(&mut self, idx: usize) {
        let (l_index, r_index) = (Self::left_child_idx(idx), Self::right_child_idx(idx));
        if idx >= self.length || l_index >= self.length {
            return;
        }

        let val = &self.data[idx];
        self.lookup.insert(val.lookup_key(), idx);
        let lval = &self.data[l_index];
        self.lookup.insert(lval.lookup_key(), l_index);
        let get_min = |me: &T, other: &T| -> T {
            match me.partial_cmp(other).expect("failed to get ordering") {
                std::cmp::Ordering::Less => val,
                std::cmp::Ordering::Greater | std::cmp::Ordering::Equal => other,
            }
            .to_owned()
        };

        let mut min = get_min(&val, lval);
        if let Some(rval) = self.data.get(r_index) {
            self.lookup.insert(rval.lookup_key(), r_index);
            min = get_min(&min, rval);
        }

        match min {
            _ if min == *val => {
                // All is well if parent is min
            }
            _ if min == *lval => {
                self.heapify_down(l_index);
                self.swap(idx, l_index);
            }
            _ => {
                // min must be rval
                self.heapify_down(r_index);
                self.swap(idx, r_index);
            }
        }
    }

    fn heapify_up(&mut self, idx: usize) {
        if idx == 0 {
            return;
        }
        let parent_idx = Self::parent_idx(idx);
        let parent_val = self.data[parent_idx].clone();
        self.lookup.insert(parent_val.lookup_key(), parent_idx);
        let val = self.data[idx].clone();
        self.lookup.insert(val.lookup_key(), idx);

        if parent_val > val {
            self.swap(idx, parent_idx);
            self.heapify_up(parent_idx)
        }
    }

    fn parent_idx(idx: usize) -> usize {
        // Numbers that don't evenly go into 2 return the division just without a remainder, not
        // floating point numbers
        (idx - 1) / 2
    }

    fn left_child_idx(idx: usize) -> usize {
        idx * 2 + 1
    }

    fn right_child_idx(idx: usize) -> usize {
        idx * 2 + 2
    }
}

mod tests {
    use std::collections::HashMap;

    use libp2p::{identity::Keypair, PeerId};

    use crate::chain::transaction::PendingTransaction;

    use super::{MinHeapMap, MinMapHeapable};

    fn create_heap_map(ids: &[PeerId]) -> MinHeapMap<PeerId, PendingTransaction> {
        let mut heap_vec = Vec::new();
        for id in ids {
            let tx = PendingTransaction::new(*id, String::new());
            heap_vec.push(tx);
        }
        MinHeapMap::from(heap_vec)
    }

    #[test]
    fn swap_works() {
        let input = [0., 32., 65., 16.];

        let mut ids = vec![];
        for _ in 0..input.len() {
            let keys = Keypair::generate_ed25519();
            ids.push(PeerId::from(keys.public()));
        }
        let mut heap_map = create_heap_map(&ids);

        let key0 = heap_map.lookup.get(&ids[0]).unwrap().clone();
        let key1 = heap_map.lookup.get(&ids[1]).unwrap().clone();
        heap_map.swap(key0, key1);

        let mut expected_map = heap_map.lookup.clone();

        expected_map.insert(ids[0], key1);
        expected_map.insert(ids[1], key0);

        assert_eq!(heap_map.lookup, expected_map);
    }

    #[test]
    fn heap_works() {
        let mut ids = vec![];
        let mut heap_map = MinHeapMap::new();
        for _ in 0..5 {
            let keys = Keypair::generate_ed25519();
            let id = PeerId::from(keys.public());
            ids.push(id);
            heap_map.insert(PendingTransaction::new(id, String::new()));
        }

        assert_eq!(heap_map.pop().unwrap().client, ids[0]);
        assert_eq!(heap_map.length, 4);
        assert_eq!(heap_map.pop().unwrap().client, ids[1]);

        let id_to_mut = *heap_map.lookup.keys().next().unwrap();
        heap_map
            .lookup_and_mutate(id_to_mut, |v| v.client = ids[3])
            .unwrap();

        assert_eq!(heap_map.lookup(id_to_mut).unwrap().client, ids[3]);
    }
}
