use super::heap::max::MaxHeapable;
use libp2p::{
    gossipsub::{IdentTopic, TopicHash},
    PeerId,
};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

pub const IDENTIFY_ID: &str = "/id/1.0.0";
pub type NetworkReqRes = libp2p::request_response::json::Behaviour<NetworkRequest, NetworkResponse>;

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub enum NetworkRequest {
    // Chain,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub enum NetworkResponse {
    // Chain(Blockchain),
}

pub enum NetworkTopic<'t> {
    /// All validators subscribe to pending topic, everyone else need only publish
    PendingTx,
    /// All nodes subscribe to this topic, only validators publish
    ChainUpdate,
    /// All providers subscribe to Auction topic, clients need only to publish
    Auction,
    /// Clients each subscribe to their own topic, providers publish when bidding
    Client(&'t PeerId),
}

impl<'t> From<&'t PeerId> for NetworkTopic<'t> {
    fn from(value: &'t PeerId) -> Self {
        Self::Client(value)
    }
}

impl<'t> NetworkTopic<'t> {
    const AUCTION: &'t str = "auction";
    const PENDING_TX: &'t str = "pending";
    const CHAIN_UPDATE: &'t str = "chain_update";
    pub fn publish(&self) -> TopicHash {
        match self {
            Self::Auction => TopicHash::from_raw(Self::AUCTION),
            Self::PendingTx => TopicHash::from_raw(Self::PENDING_TX),
            Self::ChainUpdate => TopicHash::from_raw(Self::CHAIN_UPDATE),
            Self::Client(peer) => TopicHash::from_raw(peer.to_string()),
        }
    }

    pub fn subscribe(&self) -> IdentTopic {
        match self {
            Self::Auction => IdentTopic::new(Self::AUCTION),
            Self::PendingTx => IdentTopic::new(Self::PENDING_TX),
            Self::ChainUpdate => IdentTopic::new(Self::CHAIN_UPDATE),
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
