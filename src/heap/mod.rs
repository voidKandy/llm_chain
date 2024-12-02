pub mod max;
pub mod min_map;

#[derive(Debug)]
pub enum HeapError {
    LengthIsZero,
    LookupReturnedNone,
}
