use std::{cmp::Ordering, sync::LazyLock};

use libp2p::{
    gossipsub::{IdentTopic, TopicHash},
    PeerId,
};
use serde::{Deserialize, Serialize};

use crate::heap::max::MaxHeapable;
pub type CompReqRes = libp2p::request_response::json::Behaviour<CompConnect, CompConnectConfirm>;

// should send encryption key
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct CompConnect {
    pub tokens: f64,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct CompConnectConfirm {
    pub ok: bool,
}

pub enum SysTopic<'t> {
    Pending,
    Completed,
    Client(&'t PeerId),
}

impl<'t> From<&'t PeerId> for SysTopic<'t> {
    fn from(value: &'t PeerId) -> Self {
        Self::Client(value)
    }
}

impl<'t> SysTopic<'t> {
    const PENDING: &'t str = "tx_pending";
    const COMPLETED: &'t str = "tx_completed";
    pub fn publish(&self) -> TopicHash {
        match self {
            Self::Pending => TopicHash::from_raw(Self::PENDING),
            Self::Completed => TopicHash::from_raw(Self::COMPLETED),
            Self::Client(peer) => TopicHash::from_raw(peer.to_string()),
        }
    }

    pub fn subscribe(&self) -> IdentTopic {
        match self {
            Self::Pending => IdentTopic::new(Self::PENDING),
            Self::Completed => IdentTopic::new(Self::COMPLETED),
            Self::Client(peer) => IdentTopic::new(peer.to_string()),
        }
    }
    // pub const PENDING: LazyLock<TopicHash> = LazyLock::new(|| TopicHash::from_raw("tx_pending"));
    // pub const COMPLETED: LazyLock<TopicHash> =
    //     LazyLock::new(|| TopicHash::from_raw("tx_completed"));
    // pub fn client(client_id: &PeerId) -> TopicHash {
    //     TopicHash::from_raw(client_id.to_string())
    // }
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
