use crate::util::{
    map_vec::{Contains, MapVec},
    PublicKeyBytes,
};
use serde::{Deserialize, Serialize};
use sha3::Digest;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Transfer {
    pub(super) hash: String,
    pub(super) timestamp: String,
    pub(super) sender: PublicKeyBytes,
    pub(super) receiver: PublicKeyBytes,
    pub(super) tokens: f64,
    pub(super) inputs: Vec<String>,
    pub(super) outputs: MapVec<String, super::UTXO>,
    pub(super) signature: Option<Vec<u8>>,
}

impl Contains<String> for Transfer {
    fn get_ref(&self) -> &String {
        &self.hash
    }
}

pub struct Fields<'h> {
    timestamp: &'h str,
    sender: &'h PublicKeyBytes,
    receiver: &'h PublicKeyBytes,
    tokens: &'h f64,
    inputs: &'h [String],
    outputs: &'h [super::UTXO],
}

impl<'h> From<&'h Transfer> for Fields<'h> {
    fn from(value: &'h Transfer) -> Self {
        Self {
            timestamp: &value.timestamp,
            sender: &value.sender,
            receiver: &value.receiver,
            tokens: &value.tokens,
            inputs: &value.inputs,
            outputs: &value.outputs.as_ref(),
        }
    }
}

impl<'h> crate::util::hash::Hash<'h> for Transfer {
    type Fields = Fields<'h>;
    fn hash_ref(&self) -> &str {
        &self.hash
    }
    fn hash_fields(fields: Self::Fields) -> sha3::digest::Output<crate::util::hash::Hasher> {
        let mut hasher = Self::hasher();
        hasher.update(fields.timestamp);
        hasher.update(fields.sender.as_ref());
        hasher.update(fields.receiver.as_ref());
        hasher.update(fields.tokens.to_string());
        Self::update_multiple(&mut hasher, fields.inputs);
        Self::update_multiple(&mut hasher, fields.outputs);
        hasher.finalize()
    }
}
