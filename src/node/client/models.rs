use crate::{behavior::gossip::ProvisionBid, heap::max::MaxHeap};
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader, Lines, Stdin};

pub(super) const AUCTIONING_DURATION: Duration = Duration::from_millis(100);
#[derive(Debug)]
pub(super) enum ClientNodeState {
    Idle {
        stdin: Lines<BufReader<Stdin>>,
    },
    Auctioning {
        start: Instant,
        bids: MaxHeap<ProvisionBid>,
    },
    Connecting {
        sent_request: bool,
        bid: ProvisionBid,
        provider: PeerId,
    },
    GettingCompletion {
        provider: PeerId,
        expected_amt_messages: Option<usize>,
        messages: Vec<(usize, String)>,
    },
}

#[derive(Debug)]
pub(super) enum StateEvent {
    UserInput(String),
    ChoseBid(ProvisionBid),
    GotCompletion { provider: PeerId, content: String },
}

impl Default for ClientNodeState {
    fn default() -> Self {
        Self::Idle {
            stdin: tokio::io::BufReader::new(tokio::io::stdin()).lines(),
        }
    }
}

// this should be done as state, just as the provider is
#[derive(Debug, Deserialize, Serialize)]
pub(super) enum ClientChannelMessage {
    Completion(CompletionMessage),
    Bid(ProvisionBid),
}

impl Into<ClientChannelMessage> for CompletionMessage {
    fn into(self) -> ClientChannelMessage {
        ClientChannelMessage::Completion(self)
    }
}

impl Into<ClientChannelMessage> for ProvisionBid {
    fn into(self) -> ClientChannelMessage {
        ClientChannelMessage::Bid(self)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum CompletionMessage {
    Working { idx: usize, token: String },
    Finished { peer: PeerId, total_messages: usize },
}
