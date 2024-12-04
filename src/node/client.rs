use core::panic;
use std::time::{Duration, Instant};

use super::{Node, NodeType, Wallet};
use crate::{
    behavior::{
        gossip::{CompConnect, ProvisionBid, SysTopic},
        SysBehaviour, SysBehaviourEvent,
    },
    chain::transaction::PendingTransaction,
    heap::max::MaxHeap,
    MainResult,
};
use futures::StreamExt;
use libp2p::{
    gossipsub::{self, TopicHash},
    request_response,
    swarm::SwarmEvent,
    PeerId, Swarm,
};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader, Lines, Stdin};
use tracing::warn;

#[derive(Debug)]
pub struct ClientNode {
    my_topic: TopicHash,
    wallet: Wallet,
    state: ClientNodeState,
}

const AUCTIONING_DURATION: Duration = Duration::from_millis(100);
#[derive(Debug)]
enum ClientNodeState {
    Idle {
        stdin: Lines<BufReader<Stdin>>,
    },
    Auctioning {
        start: Instant,
        bids: MaxHeap<ProvisionBid>,
    },
    Connecting {
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
enum StateEvent {
    UserInput(String),
    ChoseBid(ProvisionBid),
    GotCompletion { provider: PeerId, content: String },
}

impl ClientNodeState {
    async fn try_next_state_event(&mut self) -> anyhow::Result<Option<StateEvent>> {
        match self {
            Self::Idle { stdin } => {
                if let Some(input) = stdin.next_line().await? {
                    if !input.is_empty() {
                        return Ok(Some(StateEvent::UserInput(input)));
                    }
                }
            }

            Self::Auctioning { start, bids } => {
                let elapsed = start.elapsed();
                tracing::warn!("elapsed: {elapsed:#?}");
                if elapsed >= AUCTIONING_DURATION && bids.len() > 0 {
                    tracing::warn!("choosing bid");
                    let bid = bids.pop().expect("failed to get bid from heap");
                    return Ok(Some(StateEvent::ChoseBid(bid)));
                }
            }

            Self::GettingCompletion {
                provider,
                expected_amt_messages,
                messages,
            } => {
                if let Some(exp) = expected_amt_messages {
                    if *exp == messages.len() {
                        let content = messages
                            .iter()
                            .fold(String::new(), |acc, (_, m)| format!("{acc}{m}"));
                        return Ok(Some(StateEvent::GotCompletion {
                            provider: *provider,
                            content,
                        }));
                    }
                }
            }
            _ => {}
        }
        Ok(None)
    }
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
enum ClientChannelMessage {
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

impl<'w> NodeType<'w> for ClientNode {
    fn wallet_val(&'w mut self) -> &'w mut Wallet {
        &mut self.wallet
    }
    fn init(swarm: &mut Swarm<SysBehaviour>) -> MainResult<Self> {
        let my_topic = TopicHash::from_raw(swarm.local_peer_id().to_string());
        let topic = gossipsub::IdentTopic::new(my_topic.to_string());
        swarm
            .behaviour_mut()
            .gossip
            .subscribe(&topic)
            .expect("client failed to subscribe to it's unique topic'");

        Ok(ClientNode {
            wallet: Wallet::new(),
            my_topic,
            state: ClientNodeState::default(),
        })
    }

    async fn loop_logic(node: &mut Node<Self>) -> MainResult<()> {
        if let ClientNodeState::Connecting { provider, bid } = &node.typ.state {
            node.swarm
                .behaviour_mut()
                .req_res
                .send_request(&provider, CompConnect { tokens: bid.bid });
        }
        tokio::select! {
            event = node.swarm.select_next_some() => Self::default_handle_swarm_event(node, event).await,
            Ok(Some(event)) = node.typ.state.try_next_state_event() => {
                warn!("handling client state event: {event:#?}");
                match event {
                    StateEvent::ChoseBid(bid) => {
                        tracing::warn!("choosing bid: {bid:#?}");
                        node.typ.state = ClientNodeState::Connecting {  provider: bid.peer, bid , };
                    },
                    StateEvent::UserInput(line) => {
                        let local_id = node.swarm.local_peer_id();
                        let tx = PendingTransaction::new(*local_id, line);
                        tracing::warn!("client publishing: {tx:#?}");
                        let data = serde_json::to_vec(&tx).unwrap();
                        node.swarm
                            .behaviour_mut()
                            .gossip
                            .publish(SysTopic::Pending.publish(), data)
                            .unwrap();
                        node.typ.state = ClientNodeState::Auctioning {
                            start: Instant::now(),
                            bids: vec![].into(),
                        };
                    }

                    StateEvent::GotCompletion { provider, content } => {
                        warn!("got completion: {content}");
                        node.typ.state = ClientNodeState::default();
                    },
                }
                Ok(())
            }
        }
    }

    async fn handle_swarm_event(
        node: &mut Node<Self>,
        event: SwarmEvent<SysBehaviourEvent>,
    ) -> MainResult<()> {
        match event {
            SwarmEvent::Behaviour(SysBehaviourEvent::Gossip(gossipsub::Event::Message {
                message: gossipsub::Message { data, topic, .. },
                ..
            })) if topic == node.typ.my_topic => {
                let message: ClientChannelMessage = {
                    if let Ok(bid) = serde_json::from_slice::<ProvisionBid>(&data) {
                        bid.into()
                    } else if let Ok(mes) = serde_json::from_slice::<CompletionMessage>(&data) {
                        mes.into()
                    } else {
                        panic!(
                            "recieved message that could not be coerced into ClientChannelMessage"
                        );
                    }
                };

                warn!("client received: {message:#?}");

                match message {
                    ClientChannelMessage::Completion(comp) => {
                        if let ClientNodeState::GettingCompletion {
                            provider,
                            expected_amt_messages,
                            messages,
                        } = &mut node.typ.state
                        {
                            match comp {
                                CompletionMessage::Finished {
                                    peer,
                                    total_messages,
                                } => {
                                    assert_eq!(
                                        *provider, peer,
                                        "somehow completion was signed with wrong signature"
                                    );
                                    *expected_amt_messages = Some(total_messages);
                                }
                                CompletionMessage::Working { idx, token } => {
                                    messages.push((idx, token));
                                }
                            }
                        }
                    }

                    ClientChannelMessage::Bid(bid) => {
                        if let ClientNodeState::Auctioning { ref mut bids, .. } = node.typ.state {
                            bids.insert(bid);
                        }
                    }
                }
            }

            SwarmEvent::Behaviour(SysBehaviourEvent::ReqRes(
                request_response::Event::Message {
                    peer,
                    message:
                        request_response::Message::Response {
                            request_id,
                            response,
                        },
                },
            )) => {
                if response.ok {
                    node.typ.state = ClientNodeState::GettingCompletion {
                        provider: peer,
                        expected_amt_messages: None,
                        messages: vec![],
                    };
                } else {
                    panic!("got not okay response from provider!!");
                }
            }

            _ => {
                warn!("unhandled client event: {event:#?}");
            }
        }
        Ok(())
    }
}
