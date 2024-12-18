use serde::Serialize;
use sha3::{
    digest::{core_api::CoreWrapper, Output},
    Digest, Sha3_256, Sha3_256Core,
};

use crate::MainResult;

pub type Hasher = CoreWrapper<Sha3_256Core>;

pub trait Hash<'h>
where
    Self: 'h,
{
    type Fields: From<&'h Self>;
    fn hash_ref(&self) -> &str;
    fn hash_fields(fields: Self::Fields) -> Output<Hasher>;
    fn valid(&self) -> bool {
        self.hash_ref() == Self::output_to_string(self.my_hash())
    }
    fn my_hash(&self) -> Output<Hasher> {
        let fields = Self::Fields::from(self);
        Self::hash_fields(fields)
    }
    fn hasher() -> Hasher {
        Sha3_256::new()
    }
    fn output_to_string(output: Output<Hasher>) -> String {
        format!("{output:x}")
    }
    fn update_multiple<T>(hasher: &'h mut Hasher, many: impl Into<&'h [T]>) -> MainResult<()>
    where
        T: Serialize + 'h,
    {
        for v in Into::<&[T]>::into(many) {
            let serialized = serde_json::to_string(v)?;
            hasher.update(serialized);
        }
        Ok(())
    }
}
