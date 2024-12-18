use super::HeapError;

#[derive(Debug)]
pub struct MaxHeap<T> {
    data: Vec<T>,
    length: usize,
}

pub trait MaxHeapable: PartialEq + PartialOrd + Clone + std::fmt::Debug {}

impl<T: MaxHeapable> From<Vec<T>> for MaxHeap<T> {
    fn from(value: Vec<T>) -> Self {
        let mut new = Self::new();
        for v in value {
            new.insert(v);
        }

        new
    }
}

impl<T: MaxHeapable> MaxHeap<T> {
    pub fn new() -> Self {
        Self {
            data: vec![],
            length: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.length
    }

    pub fn insert(&mut self, val: T) {
        self.data.push(val);
        self.heapify_up(self.length);
        self.length += 1;
    }

    pub fn peek(&self) -> Option<&T> {
        self.data.get(0)
    }

    pub fn pop(&mut self) -> Result<T, HeapError> {
        if self.length == 0 {
            return Err(HeapError::LengthIsZero);
        }
        let out = self.data.remove(0);
        self.length -= 1;

        if self.length == 0 {
            self.data = vec![];
            self.length = 0;
            return Ok(out);
        }
        println!("out: {out:#?}");
        self.heapify_down(0);
        Ok(out)
    }

    fn swap(&mut self, one: usize, other: usize) {
        let one_val = self.data[one].clone();
        self.data[one] = self.data[other].to_owned();
        self.data[other] = one_val;
    }

    fn heapify_down(&mut self, idx: usize) {
        let (l_index, r_index) = (Self::left_child_idx(idx), Self::right_child_idx(idx));
        if idx >= self.length || l_index >= self.length {
            return;
        }

        let val = &self.data[idx];
        let lval = &self.data[l_index];

        let get_max = |me: &T, other: &T| -> T {
            match me.partial_cmp(other).expect("failed to get ordering") {
                std::cmp::Ordering::Less => other,
                std::cmp::Ordering::Greater | std::cmp::Ordering::Equal => me,
            }
            .to_owned()
        };

        let mut max = get_max(&val, lval);
        // println!("got max: {max:#?} out of {val:#?} & {lval:#?}");
        if let Some(rval) = self.data.get(r_index) {
            max = get_max(&max, rval);
        }

        match max {
            _ if max == *val => {
                // All is well if parent is max
            }
            _ if max == *lval => {
                self.heapify_down(l_index);
                self.swap(idx, l_index);
            }
            _ => {
                // max must be rval
                self.heapify_down(r_index);
                self.swap(idx, r_index);
            }
        }
        println!("heapified: {self:#?}");
    }

    fn heapify_up(&mut self, idx: usize) {
        if idx == 0 {
            return;
        }
        let parent_idx = Self::parent_idx(idx);
        let parent_val = &self.data[parent_idx];
        let val = &self.data[idx];

        if parent_val < val {
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

    use libp2p::{identity::Keypair, PeerId};

    use crate::util::behaviour::ProvisionBid;

    use super::MaxHeap;

    fn create_heap(ids: &[PeerId], bids: &[f64]) -> MaxHeap<ProvisionBid> {
        assert!(ids.len() == bids.len());
        let mut heap_vec = Vec::new();
        for i in 0..bids.len() {
            let bid = ProvisionBid::new(ids[i], 55, bids[i]);
            // let tx = PendingTransaction::new(ids[i], bids[i], String::new());
            heap_vec.push(bid);
        }
        MaxHeap::from(heap_vec)
    }

    #[test]
    fn swap_works() {
        let input = [0., 32., 65., 16.];

        let mut ids = vec![];
        for _ in 0..input.len() {
            let keys = Keypair::generate_ed25519();
            ids.push(PeerId::from(keys.public()));
        }
        let mut heap = create_heap(&ids, &input);

        let expected = heap.data[2].clone();

        heap.swap(1, 2);

        assert_eq!(heap.data[1].bid, expected.bid);
    }

    #[test]
    fn heap_works() {
        let input = [0., 32., 65., 16., 19., 12., 14., 7., 8.];

        // let mut ids = vec![];
        let mut heap = MaxHeap::new();
        for i in 0..input.len() {
            let keys = Keypair::generate_ed25519();
            let id = PeerId::from(keys.public());
            // ids.push(id);
            heap.insert(ProvisionBid::new(id, 55, input[i]));
        }

        assert_eq!(heap.pop().unwrap().bid, 65.);
        assert_eq!(heap.length, 8);
        assert_eq!(heap.pop().unwrap().bid, 32.);
    }
}
