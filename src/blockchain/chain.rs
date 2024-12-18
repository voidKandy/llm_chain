use std::sync::LazyLock;

use crate::util::map_vec::{Contains, MapVec};

use super::{
    block::Block,
    transaction::{update_multiple, Input, Output, PublicKeyBytes, Transaction},
};
use chrono::Utc;
use libp2p::identity::{ed25519::PublicKey, PublicKey};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

impl Contains<String> for Block {
    fn get_ref(&self) -> &String {
        &self.hash
    }
    fn get_mut(&mut self) -> &mut String {
        &mut self.hash
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Blockchain {
    data: MapVec<String, Block>,
}

const GENESIS_BLOCK: LazyLock<Block> =
    LazyLock::new(|| Block::new(0, String::new(), vec![], Utc::now().to_rfc3339()));

impl Blockchain {
    pub fn new() -> Blockchain {
        let block = LazyLock::force(&GENESIS_BLOCK).clone();
        let mut chain = Blockchain {
            data: MapVec::new(),
        };
        chain.data.push(block);
        chain
    }

    pub fn validate(&self) -> bool {
        self.data.iter_vals().all(|b| b.validate())
    }

    // pub fn get_output_amt(&self, input: &Input) -> Option<f64> {
    // self.tx_lookup
    //     .get(&input.transaction_id)
    //     .and_then(|(block_idx, tx_idx)| {
    //         Some(self.data[*block_idx].transactions[*tx_idx].tokens())
    //     })
    // }

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
