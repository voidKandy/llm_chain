use std::sync::LazyLock;

use crate::util::map_vec::{Contains, MapVec};

use super::transaction::{update_multiple, Input, Output, PublicKeyBytes, Transaction};
use chrono::Utc;
use libp2p::identity::{ed25519::PublicKey, PublicKey};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Block {
    pub index: u64,                                // Block index in the chain.
    pub timestamp: String,                         // Timestamp of block creation.
    pub previous_hash: String,                     // Hash of the previous block.
    pub nonce: u64,                                // Nonce for Proof-of-Work.
    pub hash: String,                              // Hash of the block.
    pub transactions: MapVec<String, Transaction>, // Transactions included in the block.
    // pub mint: Mint,
    pub signature: Option<Vec<u8>>,
}

pub struct Fields<'h> {
    index: &'h u64,
    timestamp: &'h str,
    previous_hash: &'h str,
    nonce: &'h u64,
    hash: &'h str,
    mint: &'h Mint,
}

impl Contains<String> for Transaction {
    fn get_ref(&self) -> &String {
        &self.hash
    }
    fn get_mut(&mut self) -> &mut String {
        &mut self.hash
    }
}

impl Block {
    /// Creates a new block.
    pub fn new(
        index: u64,
        previous_hash: String,
        transactions: Vec<Transaction>,
        timestamp: String,
    ) -> Self {
        let mut block = Block {
            index,
            timestamp,
            previous_hash,
            nonce: 0,
            hash: String::new(),
            transactions,
        };
        block.hash = block.calculate_hash();
        block
    }

    /// Calculates the block's hash.
    fn calculate_hash(&self) -> String {
        let mut hasher = Sha3_256::new();
        hasher.update(self.index.to_string());
        hasher.update(&self.timestamp);
        hasher.update(&self.previous_hash);
        hasher.update(self.nonce.to_string());
        for tx in &self.transactions {
            hasher.update(tx.hash());
        }
        format!("{:x}", hasher.finalize())
    }

    /// Mines the block using a Proof-of-Work mechanism.
    fn mine(&mut self, difficulty: usize) {
        let target = "0".repeat(difficulty);
        while &self.hash[0..difficulty] != target {
            self.nonce += 1;
            self.hash = self.calculate_hash();
        }
    }

    /// Validates the block's integrity.
    fn validate(&self) -> bool {
        // Check if the block's hash matches its contents.
        if self.hash != self.calculate_hash() {
            return false;
        }
        // Validate all transactions in the block.
        self.transactions
            .iter()
            .all(|tx| tx.validate_hash() && tx.verify_signature().is_some_and(|b| b))
    }
}
