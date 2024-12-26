pub mod behaviour;
pub mod hash;
pub mod heap;
// pub mod json_rpc;
pub mod map_vec;
use chrono::Utc;
use libp2p::{identity::PublicKey, StreamProtocol};

pub enum OneOf<T, O> {
    Left(T),
    Right(O),
}

pub fn now_timestamp_string() -> String {
    Utc::now().to_rfc2822()
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
pub struct PublicKeyBytes(Vec<u8>);

impl AsRef<Vec<u8>> for PublicKeyBytes {
    fn as_ref(&self) -> &Vec<u8> {
        &self.0
    }
}

impl From<PublicKey> for PublicKeyBytes {
    fn from(value: PublicKey) -> Self {
        Self(value.encode_protobuf())
    }
}

impl TryInto<PublicKey> for &PublicKeyBytes {
    type Error = libp2p::identity::DecodingError;
    fn try_into(self) -> Result<PublicKey, Self::Error> {
        PublicKey::try_decode_protobuf(&self.0)
    }
}
