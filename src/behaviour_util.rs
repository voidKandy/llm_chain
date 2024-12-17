use std::{cmp::Ordering, sync::LazyLock};

use crate::{chain::block::Blockchain, heap::max::MaxHeapable};
use libp2p::{
    gossipsub::{IdentTopic, TopicHash},
    PeerId, StreamProtocol,
};
use serde::{Deserialize, Serialize};

pub const IDENTIFY_ID: &str = "/id/1.0.0";
pub type NetworkReqRes = libp2p::request_response::json::Behaviour<NetworkRequest, NetworkResponse>;

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub enum NetworkRequest {
    Chain,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub enum NetworkResponse {
    Chain(Blockchain),
}

pub enum NetworkTopic<'t> {
    // All providers subscribe to Auction topic
    Auction,
    Pending,
    Completed,
    Client(&'t PeerId),
}

impl<'t> From<&'t PeerId> for NetworkTopic<'t> {
    fn from(value: &'t PeerId) -> Self {
        Self::Client(value)
    }
}

impl<'t> NetworkTopic<'t> {
    const AUCTION: &'t str = "auction";
    const PENDING: &'t str = "tx_pending";
    const COMPLETED: &'t str = "tx_completed";
    pub fn publish(&self) -> TopicHash {
        match self {
            Self::Auction => TopicHash::from_raw(Self::AUCTION),
            Self::Pending => TopicHash::from_raw(Self::PENDING),
            Self::Completed => TopicHash::from_raw(Self::COMPLETED),
            Self::Client(peer) => TopicHash::from_raw(peer.to_string()),
        }
    }

    pub fn subscribe(&self) -> IdentTopic {
        match self {
            Self::Auction => IdentTopic::new(Self::AUCTION),
            Self::Pending => IdentTopic::new(Self::PENDING),
            Self::Completed => IdentTopic::new(Self::COMPLETED),
            Self::Client(peer) => IdentTopic::new(peer.to_string()),
        }
    }
}

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
