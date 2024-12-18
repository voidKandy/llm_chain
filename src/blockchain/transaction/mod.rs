use crate::util::{hash::Hash, map_vec::Contains, PublicKeyBytes};
use sha3::Digest;

pub mod mint;
pub mod transfer;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct UTXO {
    hash: String,
    amount: f64,
    receiver: crate::util::PublicKeyBytes,
}

impl Contains<PublicKeyBytes> for UTXO {
    fn get_ref(&self) -> &PublicKeyBytes {
        &self.receiver
    }
}

impl<'h> From<&'h UTXO> for Fields<'h> {
    fn from(value: &'h UTXO) -> Self {
        Self {
            amount: &value.amount,
            receiver: &value.receiver,
        }
    }
}

pub struct Fields<'h> {
    amount: &'h f64,
    receiver: &'h crate::util::PublicKeyBytes,
}

impl<'h> Hash<'h> for UTXO {
    type Fields = Fields<'h>;
    fn hash_ref(&self) -> &str {
        &self.hash
    }
    fn hash_fields(fields: Self::Fields) -> sha3::digest::Output<crate::util::hash::Hasher> {
        let mut hasher = Self::hasher();
        hasher.update(fields.amount.to_string());
        hasher.update(fields.receiver.as_ref());
        hasher.finalize()
    }
}

impl crate::util::map_vec::Contains<String> for UTXO {
    fn get_ref(&self) -> &String {
        &self.hash
    }
}

impl UTXO {
    pub fn new(amount: f64, pub_key: impl Into<PublicKeyBytes>) -> Self {
        let receiver = Into::<PublicKeyBytes>::into(pub_key);
        let fields = Fields {
            amount: &amount,
            receiver: &receiver,
        };
        let hash = Self::output_to_string(Self::hash_fields(fields));
        Self {
            hash,
            amount,
            receiver,
        }
    }
}
