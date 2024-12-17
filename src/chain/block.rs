use std::sync::LazyLock;

use super::transaction::{update_multiple, Input, Output, PublicKeyBytes, Transaction};
use chrono::Utc;
use libp2p::identity::PublicKey;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Block {
    pub index: u64,            // Block index in the chain.
    pub timestamp: String,     // Timestamp of block creation.
    pub previous_hash: String, // Hash of the previous block.
    pub nonce: u64,            // Nonce for Proof-of-Work.
    // this could be sized
    pub hash: String,                   // Hash of the block.
    pub transactions: Vec<Transaction>, // Transactions included in the block.
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Blockchain {
    data: Vec<Block>,
    block_lookup: std::collections::HashMap<String, usize>,
    // val is the idx of the block, and then the idx of the tx in that block
    tx_lookup: std::collections::HashMap<String, (usize, usize)>,
}

const GENESIS_BLOCK: LazyLock<Block> =
    LazyLock::new(|| Block::new(0, String::new(), vec![], Utc::now().to_rfc3339()));

impl Blockchain {
    pub fn new() -> Blockchain {
        let block = LazyLock::force(&GENESIS_BLOCK).clone();
        let mut chain = Blockchain {
            data: vec![],
            block_lookup: std::collections::HashMap::new(),
            tx_lookup: std::collections::HashMap::new(),
        };
        chain.push(block);
        chain
    }

    pub fn push(&mut self, block: Block) {
        let idx = self.data.len();
        if idx != 0 {
            assert!(
                self.data
                    .get(idx - 1)
                    .is_some_and(|b| b.hash == block.previous_hash),
                "invalid previous hash"
            );
        }
        let block_hash = block.hash.to_owned();
        self.data.push(block);
        for (i, tx) in self.data[idx].transactions.iter().enumerate() {
            self.tx_lookup.insert(tx.hash().to_string(), (idx, i));
        }
        self.block_lookup.insert(block_hash, idx);
    }

    pub fn validate(&self) -> bool {
        self.data.iter().all(|b| b.validate())
    }

    pub fn get_output_amt(&self, input: &Input) -> Option<f64> {
        self.tx_lookup
            .get(&input.transaction_id)
            .and_then(|(block_idx, tx_idx)| {
                Some(self.data[*block_idx].transactions[*tx_idx].tokens())
            })
    }

    pub fn new_transaction(
        &self,
        timestamp: String,
        sender: PublicKey,
        receiver: PublicKey,
        tokens: f64,
        inputs: Vec<Input>,
        // outputs: Vec<Output>,
    ) -> Transaction {
        let sender: PublicKeyBytes = sender.into();
        let receiver: PublicKeyBytes = receiver.into();

        // Calculate total input value (sum of all UTXOs referenced by inputs)
        let total_input_value = inputs.iter().fold(0., |sum, input| {
            sum + self
                .get_output_amt(input)
                .expect("input points to non-existant output")
        });

        // Determine the change (if any)
        let change = total_input_value - tokens;

        // Create the outputs (receiver + change)
        let mut outputs = vec![Output {
            receiver: receiver.clone(),
            amount: tokens,
        }];

        if change > 0.0 {
            outputs.push(Output {
                receiver: sender.clone(), // The sender receives the change
                amount: change,
            });
        }

        let mut hasher = Sha3_256::new();
        hasher.update(&timestamp);
        hasher.update(&sender.as_ref());
        hasher.update(&receiver.as_ref());
        hasher.update(tokens.to_string());
        update_multiple(&mut hasher, &inputs).unwrap();
        // update_multiple(&mut hasher, &outputs);
        let hash = format!("{:x}", hasher.finalize());

        Transaction {
            hash,
            timestamp,
            sender,
            receiver,
            tokens,
            inputs,
            outputs,
            signature: None,
        }
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
