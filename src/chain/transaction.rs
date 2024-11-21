use std::str::FromStr;

use libp2p::{Multiaddr, PeerId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Transaction {
    sender: Multiaddr,
    receiver: Multiaddr,
    tokens: f64,
    status: TransactionStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TransactionStatus {
    Pending,
    Finished {
        // provider signature
        // should be PeerID, but it does not impl Deserialize
        sig: String,
    },
}

impl TransactionStatus {
    fn sig(&self) -> Option<PeerId> {
        if let Self::Finished { sig } = self {
            return Some(PeerId::from_str(sig).expect("could not get peer id from sig str"));
        }
        None
    }
}
