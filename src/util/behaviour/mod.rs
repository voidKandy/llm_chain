pub mod gossip;
pub mod req_res;
pub mod streaming;
use super::heap::max::MaxHeapable;
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

pub const IDENTIFY_ID: &str = "/id/1.0.0";
/// Sent by provider to request that it provide to client
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct ProvisionBid {
    pub peer: PeerId,
    distance: u64,
    pub bid: f64,
}

impl MaxHeapable for ProvisionBid {}
/// Should be changed later to account for many other factors
impl PartialOrd for ProvisionBid {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.bid.partial_cmp(&other.bid)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum BidResponse {
    Accept,
    Reject,
}

impl ProvisionBid {
    pub fn new(peer: PeerId, distance: u64, bid: f64) -> Self {
        Self {
            peer,
            distance,
            bid,
        }
    }

    pub fn better_than(&self, other: &Self) -> bool {
        // should include a distance based weight
        // should also have computer speed info
        self.bid > other.bid
    }
}
