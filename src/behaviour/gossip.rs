use libp2p::{
    gossipsub::{IdentTopic, TopicHash},
    PeerId,
};

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
