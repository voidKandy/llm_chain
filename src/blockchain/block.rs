use crate::util::{
    hash::Hash,
    map_vec::{Contains, MapVec},
};

use super::transaction::{mint::Mint, transfer::Transfer};
use libp2p::identity::{Keypair, PublicKey, SigningError};
use serde::{Deserialize, Serialize};
use sha3::Digest;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Block {
    hash: String,
    index: u64,
    timestamp: String,
    previous_hash: String,
    nonce: u64,
    transfers: MapVec<String, Transfer>,
    mint: Mint,
    signature: Vec<u8>,
}

impl Contains<String> for Block {
    fn get_ref(&self) -> &String {
        &self.hash
    }
}

impl From<Block> for UnsignedBlock {
    fn from(value: Block) -> Self {
        Self {
            hash: value.hash,
            index: value.index,
            previous_hash: value.previous_hash,
            timestamp: value.timestamp,
            nonce: value.nonce,
            transfers: value.transfers,
            mint: value.mint,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnsignedBlock {
    pub hash: String,
    index: u64,
    timestamp: String,
    previous_hash: String,
    pub nonce: u64,
    transfers: MapVec<String, Transfer>,
    mint: Mint,
}

pub struct Fields<'h> {
    index: &'h u64,
    timestamp: &'h str,
    previous_hash: &'h str,
    nonce: &'h u64,
    transfers: &'h MapVec<String, Transfer>,
    mint: &'h Mint,
}

impl<'h> From<&'h Block> for Fields<'h> {
    fn from(value: &'h Block) -> Self {
        Fields {
            index: &value.index,
            timestamp: &value.timestamp,
            previous_hash: &value.previous_hash,
            nonce: &value.nonce,
            transfers: &value.transfers,
            mint: &value.mint,
        }
    }
}

impl<'h> From<&'h UnsignedBlock> for Fields<'h> {
    fn from(value: &'h UnsignedBlock) -> Self {
        Fields {
            index: &value.index,
            timestamp: &value.timestamp,
            previous_hash: &value.previous_hash,
            nonce: &value.nonce,
            transfers: &value.transfers,
            mint: &value.mint,
        }
    }
}

impl<'h> Hash<'h> for Block {
    type Fields = Fields<'h>;
    fn hash_ref(&self) -> &str {
        &self.hash
    }
    fn hash_fields(fields: Self::Fields) -> sha3::digest::Output<crate::util::hash::Hasher> {
        <UnsignedBlock as Hash>::hash_fields(fields)
    }
}

impl<'h> Hash<'h> for UnsignedBlock {
    type Fields = Fields<'h>;
    fn hash_fields(fields: Self::Fields) -> sha3::digest::Output<crate::util::hash::Hasher> {
        let mut hasher = Self::hasher();
        assert!(
            fields.transfers.iter_vals().all(|t| t.valid()),
            "tried to hash a block with invalid transfers"
        );
        assert!(
            fields.mint.valid(),
            "tried to hash a block with an invalid mint"
        );
        hasher.update(fields.index.to_string());
        hasher.update(fields.timestamp);
        hasher.update(fields.previous_hash);
        hasher.update(fields.nonce.to_string());
        Self::update_multiple(&mut hasher, fields.transfers.as_ref());
        hasher.update(fields.mint.hash_ref());
        hasher.finalize()
    }
    fn hash_ref(&self) -> &str {
        &self.hash
    }
}

impl Block {
    /// Creates a new unsigned block & hashes
    pub fn new_unsigned(
        index: u64,
        nonce: u64,
        previous_hash: String,
        transfers: impl Into<MapVec<String, Transfer>>,
        miner_key: PublicKey,
    ) -> UnsignedBlock {
        let transfers = Into::<MapVec<String, Transfer>>::into(transfers);
        let mint = Mint::new(&transfers, miner_key);
        let timestamp = crate::util::now_timestamp_string();
        let fields = Fields {
            index: &index,
            timestamp: &timestamp,
            previous_hash: &previous_hash,
            nonce: &nonce,
            transfers: &transfers,
            mint: &mint,
        };
        let hash = Self::output_to_string(Self::hash_fields(fields));
        UnsignedBlock {
            hash,
            index,
            previous_hash,
            timestamp,
            nonce,
            transfers,
            mint,
        }
    }
}

impl UnsignedBlock {
    /// Mines the block using a Proof-of-Work mechanism.
    pub fn mine(&mut self, difficulty: usize) {
        let target = "0".repeat(difficulty);
        while &self.hash[0..difficulty] != target {
            self.nonce += 1;
            let fields = Fields::from(&*self);
            self.hash = Self::output_to_string(Self::hash_fields(fields));
        }
    }

    /// signs unsigned block and returns Block
    pub fn sign(self, keys: &Keypair) -> Result<Block, SigningError> {
        let signature = keys.sign(self.hash.as_bytes())?;
        Ok(Block {
            hash: self.hash,
            index: self.index,
            previous_hash: self.previous_hash,
            timestamp: self.timestamp,
            nonce: self.nonce,
            transfers: self.transfers,
            mint: self.mint,
            signature,
        })
    }
}
