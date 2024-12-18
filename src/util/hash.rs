use serde::Serialize;
use sha3::{
    digest::{core_api::CoreWrapper, Output},
    Digest, Sha3_256, Sha3_256Core,
};

pub type Hasher = CoreWrapper<Sha3_256Core>;

pub trait Hash<'h, F> {
    fn hasher() -> Hasher {
        Sha3_256::new()
    }
    fn hash(&self) -> &str;
    fn hash_fields(fields: F) -> Output<Hasher>;
}

pub fn update_multiple<T>(hasher: &mut Hasher, vec: &Vec<T>) -> MainResult<()>
where
    T: Serialize,
{
    for v in vec {
        let serialized = serde_json::to_string(&v)?;
        hasher.update(serialized);
    }
    Ok(())
}
