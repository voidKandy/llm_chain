use std::{io::Write, sync::LazyLock};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

use crate::MainResult;

// https://mycoralhealth.medium.com/code-a-simple-p2p-blockchain-in-go-46662601f417
// https://mycoralhealth.medium.com/code-your-own-blockchain-in-less-than-200-lines-of-go-e296282bcffc
// https://mycoralhealth.medium.com/code-your-own-blockchain-mining-algorithm-in-go-82c6a71aba1f
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Block {
    idx: usize,
    timestamp: String,
    /// this should be changed to nonce
    bpm: u64,
    hash: String,
    prev_hash: String,
}

struct BlockBuilder<'p> {
    idx: usize,
    timestamp: DateTime<Utc>,
    bpm: u64,
    prev_hash: &'p str,
}
pub type Blockchain = Vec<Block>;
pub fn init_chain() -> Blockchain {
    let gen_block = LazyLock::force(&GENESIS_BLOCK).clone();
    vec![gen_block]
}

const GENESIS_BLOCK: LazyLock<Block> = LazyLock::new(|| {
    BlockBuilder {
        timestamp: Utc::now(),
        idx: 0,
        bpm: 5,
        prev_hash: "",
    }
    .finalize()
});

/// If other chain is longer, replaces chain. Otherwise discards other
/// abstract this more generally later
fn calculate_block_hash(idx: &usize, timestamp: &str, bpm: &u64, prev_hash: &str) -> String {
    let record = format!("{}{}{}{}", idx, timestamp, bpm, prev_hash);
    let mut hasher = Sha3_256::new();
    let _ = hasher
        .write(record.as_bytes())
        .expect("failed to write to hasher buffer");
    let hash_vec = hasher.finalize();
    String::from_utf8_lossy(&hash_vec).to_string()
}

impl<'p> BlockBuilder<'p> {
    fn finalize(self) -> Block {
        let timestamp = self.timestamp.to_string();
        let hash = calculate_block_hash(&self.idx, &timestamp, &self.bpm, self.prev_hash);
        Block {
            idx: self.idx,
            timestamp,
            prev_hash: self.prev_hash.to_owned(),
            hash,
            bpm: self.bpm,
        }
    }
}

impl Block {
    fn is_valid(&self, prev: &Self) -> bool {
        if self.idx != prev.idx + 1
            || self.prev_hash != prev.hash
            || calculate_block_hash(&self.idx, &self.timestamp, &self.bpm, &self.prev_hash)
                != self.hash
        {
            return false;
        }
        true
    }

    fn generate_block<'p>(&'p self, bpm: u64) -> MainResult<Self> {
        let timestamp = Utc::now();
        let idx = self.idx + 1;
        let new_block = BlockBuilder {
            idx,
            timestamp,
            bpm,
            prev_hash: &self.hash,
        };

        Ok(new_block.finalize())
    }
}
