use crate::helpers::TEST_TRACING;
use core::util::map_vec::*;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct TestVal {
    name: String,
}

impl TestVal {
    fn new(str: &str) -> Self {
        Self {
            name: str.to_string(),
        }
    }
}

impl Contains<String> for TestVal {
    fn get_ref(&self) -> &String {
        &self.name
    }
}

const VAL1: &str = "test1";
const VAL2: &str = "test2";

#[test]
fn test_new_mapvec() {
    let mapvec: MapVec<String, TestVal> = MapVec::new();
    assert_eq!(mapvec.len(), 0);
}

#[test]
fn test_push_and_get() {
    let mut mapvec: MapVec<String, TestVal> = MapVec::new();
    let val1 = TestVal::new(VAL1);
    let val2 = TestVal::new(VAL2);

    mapvec.push(val1.clone());
    mapvec.push(val2.clone());

    assert_eq!(mapvec.len(), 2);
    assert_eq!(mapvec.get(&VAL1.to_string()), Some(&val1));
    assert_eq!(mapvec.get(&VAL2.to_string()), Some(&val2));
}

#[test]
fn test_iter_keys_and_vals() {
    let mut mapvec: MapVec<String, TestVal> = MapVec::new();
    mapvec.push(TestVal::new(VAL1));
    mapvec.push(TestVal::new(VAL2));

    let keys: Vec<String> = mapvec.iter_keys().cloned().collect();
    assert_eq!(keys, vec![VAL1.to_string(), VAL2.to_string()]);

    let vals: Vec<&TestVal> = mapvec.iter_vals().collect();
    assert_eq!(vals.len(), 2);
    assert_eq!(vals[0].name, VAL1);
    assert_eq!(vals[1].name, VAL2);
}

#[test]
fn test_remove() {
    let mut mapvec: MapVec<String, TestVal> = MapVec::new();
    mapvec.push(TestVal::new(VAL1));
    mapvec.push(TestVal::new(VAL2));

    let removed = mapvec.remove(&VAL1.to_string());
    assert_eq!(removed.unwrap().name, VAL1);
    assert_eq!(mapvec.len(), 1);
    assert!(mapvec.get(&VAL1.to_string()).is_none());
}

#[test]
fn test_peek() {
    let mut mapvec: MapVec<String, TestVal> = MapVec::new();
    mapvec.push(TestVal::new(VAL1));
    mapvec.push(TestVal::new(VAL2));

    let peeked = mapvec.peek();
    assert!(peeked.is_some());
    assert_eq!(peeked.unwrap().name, VAL2);
}

#[test]
fn test_pop() {
    LazyLock::force(&TEST_TRACING);
    let mut mapvec: MapVec<String, TestVal> = MapVec::new();
    mapvec.push(TestVal::new(VAL1));
    mapvec.push(TestVal::new(VAL2));

    let popped = mapvec.pop();
    assert!(popped.is_some());
    let (key, val) = popped.unwrap();
    assert_eq!(key, VAL2);
    assert_eq!(val.name, VAL2);
    assert_eq!(mapvec.len(), 1);

    let next_popped = mapvec.pop();
    assert!(next_popped.is_some());
    let (key, val) = next_popped.unwrap();
    assert_eq!(key, VAL1);
    assert_eq!(val.name, VAL1);
    assert_eq!(mapvec.len(), 0);

    assert!(mapvec.pop().is_none());
}
