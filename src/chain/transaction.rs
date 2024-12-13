use crate::behavior::gossip::ProvisionBid;
use chrono::{DateTime, Utc};
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::{io::Write, str::FromStr};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PendingTransaction {
    pub timestamp: String,
    pub hash: String,
    pub input: String,
    pub client: PeerId,
}

impl PartialOrd for PendingTransaction {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let my_time: DateTime<Utc> =
            DateTime::from_str(&self.timestamp).expect("couldn't serialize timestamp");
        let other_time: DateTime<Utc> =
            DateTime::from_str(&other.timestamp).expect("couldn't serialize timestamp");
        // this should return the oldest first
        my_time.partial_cmp(&other_time)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompletedTransaction {
    hash: String,
    // block_id: u64,
    timestamp: String,
    sender: PeerId,
    receiver: PeerId,
    tokens: f64,
    input: String,
    output: String,
}

impl CompletedTransaction {
    pub fn hash(&self) -> &str {
        &self.hash
    }
}

impl PendingTransaction {
    pub fn new(client: PeerId, input: String) -> Self {
        let timestamp = Utc::now().to_string();
        let record = format!("{}{}{}", timestamp, client, input);
        let mut hasher = Sha3_256::new();
        let _ = hasher
            .write(record.as_bytes())
            .expect("failed to write to hasher buffer");
        let hash_vec = hasher.finalize();
        let hash = String::from_utf8_lossy(&hash_vec).to_string();
        Self {
            input,
            hash,
            timestamp,
            client,
        }
    }

    pub fn complete(
        self,
        bid: ProvisionBid,
        receiver: PeerId,
        input: String,
        output: String,
    ) -> CompletedTransaction {
        // assert!(
        //     self.current_bid.is_none(),
        //     "Should never try to complete a pending without a bid"
        // );
        // let bid = self.current_bid.unwrap();
        let timestamp = Utc::now().to_string();
        let record = format!("{timestamp}{}{receiver}{bid:?}{input}{output}", self.client);
        let mut hasher = Sha3_256::new();
        let _ = hasher
            .write(record.as_bytes())
            .expect("failed to write to hasher buffer");
        let hash_vec = hasher.finalize();
        let hash = String::from_utf8_lossy(&hash_vec).to_string();
        CompletedTransaction {
            timestamp,
            hash,
            sender: self.client,
            tokens: bid.bid,
            receiver,
            input,
            output,
        }
    }
}
