use serde::{Deserialize, Serialize};
use std::{
    collections::{hash_map::Keys, HashMap},
    fmt, hash,
};

pub trait Contains<T> {
    fn get_ref(&self) -> &T;
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct MapVec<K: hash::Hash + Eq, T> {
    data: Vec<T>,
    map: HashMap<K, usize>,
    last_inserted: Option<K>,
}

// impl<K, T> Into<Vec<T>> for MapVec<K, T>
// where
//     T: ?Sized
//         + fmt::Debug
//         + Clone
//         + Serialize
//         + for<'de> Deserialize<'de>
//         + PartialEq
//         + Contains<K>,
//     K: fmt::Debug
//         + Clone
//         + Serialize
//         + for<'de> Deserialize<'de>
//         + PartialEq
//         + hash::Hash
//         + Eq
//         + Sized,
// {
//     fn into(self) -> Vec<T> {
//         self.data
//     }
// }

impl<K, T> AsRef<[T]> for MapVec<K, T>
where
    T: ?Sized
        + fmt::Debug
        + Clone
        + Serialize
        + for<'de> Deserialize<'de>
        + PartialEq
        + Contains<K>,
    K: fmt::Debug
        + Clone
        + Serialize
        + for<'de> Deserialize<'de>
        + PartialEq
        + hash::Hash
        + Eq
        + Sized,
{
    fn as_ref(&self) -> &[T] {
        self.data.as_ref()
    }
}

impl<K, T> From<Vec<T>> for MapVec<K, T>
where
    T: ?Sized
        + fmt::Debug
        + Clone
        + Serialize
        + for<'de> Deserialize<'de>
        + PartialEq
        + Contains<K>,
    K: fmt::Debug
        + Clone
        + Serialize
        + for<'de> Deserialize<'de>
        + PartialEq
        + hash::Hash
        + Eq
        + Sized,
{
    fn from(value: Vec<T>) -> Self {
        value.into_iter().fold(Self::new(), |mut acc, v| {
            acc.push(v);
            acc
        })
    }
}

impl<K, T> MapVec<K, T>
where
    T: ?Sized
        + fmt::Debug
        + Clone
        + Serialize
        + for<'de> Deserialize<'de>
        + PartialEq
        + Contains<K>,
    K: fmt::Debug
        + Clone
        + Serialize
        + for<'de> Deserialize<'de>
        + PartialEq
        + hash::Hash
        + Eq
        + Sized,
{
    pub fn new() -> Self {
        Self {
            data: vec![],
            last_inserted: None,
            map: HashMap::new(),
        }
    }

    pub fn iter_vals(&self) -> std::slice::Iter<'_, T> {
        self.data.iter()
    }

    pub fn iter_keys(&self) -> Keys<'_, K, usize> {
        self.map.keys()
    }

    pub fn push(&mut self, val: T) {
        let idx = self.map.len();
        self.last_inserted = Some(val.get_ref().clone());
        let _ = self.map.insert(val.get_ref().clone(), idx);
        self.data[idx] = val;
    }

    pub fn get(&self, key: &K) -> Option<&T> {
        self.map.get(key).and_then(|i| Some(&self.data[*i]))
    }

    pub fn remove(&mut self, key: &K) -> Option<T> {
        let idx = self.map.remove(key).unwrap();
        if self.last_inserted.as_ref() == Some(key) && idx > 0 {
            self.last_inserted = Some(self.data[idx - 1].get_ref().clone())
        }

        Some(self.data.remove(idx))
    }

    pub fn peek(&self) -> Option<&T> {
        self.data.last()
    }

    pub fn pop(&mut self) -> Option<(K, T)> {
        let last = self.last_inserted.take().unwrap();
        let last_idx = self.map.len() - 1;
        if last_idx > 0 {
            self.last_inserted = Some(self.data[last_idx - 1].get_ref().clone());
        }

        let val = self.data.pop();

        if val.is_some() {
            assert_eq!(last_idx, self.map.remove(&last).unwrap());
        }

        let ret = val.and_then(|v| Some((last, v)));
        ret
    }
}
