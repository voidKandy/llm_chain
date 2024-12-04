use core::panic;
use std::time::{Duration, Instant};

use super::{
    super::{Node, NodeType, Wallet},
    models::{ClientNodeState, StateEvent},
};
use crate::{
    behavior::{
        gossip::{CompConnect, ProvisionBid, SysTopic},
        SysBehaviour, SysBehaviourEvent,
    },
    chain::transaction::PendingTransaction,
    node::client::models::AUCTIONING_DURATION,
    MainResult,
};
use futures::StreamExt;
use libp2p::{
    gossipsub::{self, TopicHash},
    request_response,
    swarm::SwarmEvent,
    Swarm,
};
use tracing::warn;

#[derive(Debug)]
pub struct ClientNode {
    my_topic: TopicHash,
    wallet: Wallet,
    state: ClientNodeState,
}

impl ClientNode {
    async fn try_next_state_event(&mut self) -> anyhow::Result<Option<StateEvent>> {
        match &mut self.state {
            ClientNodeState::Idle { stdin } => {
                if let Some(input) = stdin.next_line().await? {
                    if !input.is_empty() {
                        return Ok(Some(StateEvent::UserInput(input)));
                    }
                }
            }

            ClientNodeState::Auctioning { start, bids } => {
                let elapsed = start.elapsed();
                tracing::warn!("elapsed: {elapsed:#?}");
                if elapsed >= AUCTIONING_DURATION && bids.len() > 0 {
                    if let Some(bid) = bids.peek() {
                        tracing::warn!("bid: {}, balance: {}", self.wallet.balance, bid.bid);
                        if self.wallet.balance > bid.bid {
                            let bid = bids.pop().expect("failed to get bid from heap");
                            return Ok(Some(StateEvent::ChoseBid(bid)));
                        }
                    }
                }
            }

            ClientNodeState::GettingCompletion {
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
            Ok(Some(event)) = node.typ.try_next_state_event() => {
                warn!("handling client state event: {event:#?}");
                match event {
                    StateEvent::ChoseBid(bid) => {
                        tracing::warn!("choosing bid: {bid:#?}");
                        node.typ.adjust_wallet(|w| w.balance -= bid.bid);
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
