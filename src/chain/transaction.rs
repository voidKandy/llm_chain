use chrono::{DateTime, Utc};
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::{io::Write, str::FromStr};

use crate::behavior::gossip::ProvisionBid;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PendingTransaction {
    pub timestamp: String,
    pub hash: String,
    pub input: String,
    pub client: PeerId,
    pub current_bid: Option<ProvisionBid>,
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
    timestamp: String,
    hash: String,
    client: PeerId,
    provider: PeerId,
    tokens: f64,
    input: String,
    output: String,
}

impl PendingTransaction {
    pub fn new(client: PeerId, current_bid: f64, input: String) -> Self {
        let timestamp = Utc::now().to_string();
        let record = format!("{}{}{}{}", timestamp, client, current_bid, input);
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
            current_bid: None,
        }
    }

    pub fn complete(self, provider: PeerId, input: String, output: String) -> CompletedTransaction {
        assert!(
            self.current_bid.is_none(),
            "Should never try to complete a pending without a bid"
        );
        let bid = self.current_bid.unwrap();
        let timestamp = Utc::now().to_string();
        let record = format!("{timestamp}{}{provider}{bid:?}{input}{output}", self.client,);
        let mut hasher = Sha3_256::new();
        let _ = hasher
            .write(record.as_bytes())
            .expect("failed to write to hasher buffer");
        let hash_vec = hasher.finalize();
        let hash = String::from_utf8_lossy(&hash_vec).to_string();
        CompletedTransaction {
            timestamp,
            hash,
            client: self.client,
            tokens: bid.bid,
            provider,
            input,
            output,
        }
    }
}
