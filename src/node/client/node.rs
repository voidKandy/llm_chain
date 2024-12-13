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
    node::client::models::{ClientChannelMessage, CompletionMessage, AUCTIONING_DURATION},
    MainResult,
};
use futures::StreamExt;
use libp2p::{
    gossipsub::{self, TopicHash},
    request_response,
    swarm::SwarmEvent,
    Swarm,
};
use std::time::Instant;
use tracing::warn;

#[derive(Debug)]
pub struct ClientNode {
    my_topic: TopicHash,
    wallet: Wallet,
    state: ClientNodeState,
}

impl ClientNode {
    #[allow(private_interfaces)]
    pub fn handle_event(node: &mut Node<ClientNode>, event: StateEvent) -> MainResult<()> {
        match (event, &mut node.typ.state) {
            (StateEvent::ChoseBid(bid), ClientNodeState::Auctioning { .. }) => {
                tracing::warn!("choosing bid: {bid:#?}");
                node.typ.state = ClientNodeState::Connecting {
                    sent_request: false,
                    provider: bid.peer,
                    bid,
                };
            }
            (StateEvent::UserInput(line), ClientNodeState::Idle { .. }) => {
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

            (
                StateEvent::GotCompletion { provider, content },
                ClientNodeState::GettingCompletion { .. },
            ) => {
                warn!("got completion: {content}");
                node.typ.state = ClientNodeState::default();
            }

            other => {
                warn!("no logic to handle: {other:#?}");
            }
        }
        Ok(())
    }

    async fn try_next_state_event(&mut self) -> MainResult<Option<StateEvent>> {
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
                        tracing::warn!("bid: {}, balance: {}", bid.bid, self.wallet.balance,);
                        if self.wallet.balance > bid.bid {
                            let bid = bids.pop().expect("failed to get bid from heap");
                            return Ok(Some(StateEvent::ChoseBid(bid)));
                        }
                    }
                } else {
                    warn!("still auctioning");
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
    // fn wallet_val(&'w mut self) -> &'w mut Wallet {
    //     &mut self.wallet
    // }
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
        if let ClientNodeState::Connecting {
            provider,
            bid,
            ref mut sent_request,
        } = &mut node.typ.state
        {
            if !*sent_request {
                node.swarm
                    .behaviour_mut()
                    .req_res
                    .send_request(&provider, CompConnect { tokens: bid.bid });
                *sent_request = true;
            }
        }

        tokio::select! {
            event = node.swarm.select_next_some() => Self::default_handle_swarm_event(node, event).await,
            Ok(event_opt) = node.typ.try_next_state_event() => {
                if let Some(event) = event_opt {
                    warn!("handling client state event: {event:#?}");
                    Self::handle_event(node, event)?;
                }
                Ok(())
            }
        }
    }

    async fn handle_swarm_event(
        node: &mut Node<Self>,
        event: SwarmEvent<SysBehaviourEvent>,
    ) -> MainResult<()> {
        match (event, &mut node.typ.state) {
            (
                SwarmEvent::Behaviour(SysBehaviourEvent::Gossip(gossipsub::Event::Message {
                    message: gossipsub::Message { data, topic, .. },
                    ..
                })),
                node_state,
            ) if topic == node.typ.my_topic => {
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

                match (message, node_state) {
                    (
                        ClientChannelMessage::Completion(comp),
                        ClientNodeState::GettingCompletion {
                            ref provider,
                            ref mut expected_amt_messages,
                            ref mut messages,
                        },
                    ) => match comp {
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
                    },

                    (
                        ClientChannelMessage::Bid(bid),
                        ClientNodeState::Auctioning { ref mut bids, .. },
                    ) => {
                        bids.insert(bid);
                    }

                    other => {
                        warn!(
                            "unhandled client channel message: {:#?} with state: {:#?}",
                            other.0, other.1
                        );
                    }
                }
            }

            (
                SwarmEvent::Behaviour(SysBehaviourEvent::ReqRes(
                    request_response::Event::Message {
                        peer,
                        message: request_response::Message::Response { response, .. },
                    },
                )),
                ClientNodeState::Connecting { bid, .. },
            ) => {
                if response.ok {
                    // node.typ
                    //     .publish_wallet_adjustment(&mut node.swarm, bid.to_owned());
                    node.typ.state = ClientNodeState::GettingCompletion {
                        provider: peer,
                        expected_amt_messages: None,
                        messages: vec![],
                    };
                } else {
                    // BAD! Should restart auction
                    warn!("did not receive an OK from provider, going back to idle");
                    node.typ.state = ClientNodeState::default();
                }
            }

            other => {
                warn!(
                    "unhandled swarm event: {:#?} for client state: {:#?}",
                    other.0, other.1
                );
            }
        }
        Ok(())
    }
}
