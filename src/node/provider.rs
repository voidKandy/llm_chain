use std::borrow::BorrowMut;

use super::{Node, NodeType, Wallet};
use crate::{
    behavior::{
        gossip::{self, CompConnectConfirm, SysTopic},
        SysBehaviour, SysBehaviourEvent,
    },
    chain::transaction::PendingTransaction,
    heap::min_map::{MinHeapMap, MinMapHeapable},
    node::client::models::CompletionMessage,
    MainResult,
};
use futures::StreamExt;
use libp2p::{
    gossipsub::{self},
    request_response,
    swarm::SwarmEvent,
    PeerId, Swarm,
};
use tracing::warn;

#[derive(Debug)]
pub struct ProviderNode {
    wallet: Wallet,
    pending_pool: MinHeapMap<PeerId, PendingTransaction>,
    state: ProviderNodeState,
}

#[derive(Debug, Default)]
enum ProviderNodeState {
    #[default]
    Idle,
    Bidding(PendingTransaction),
    Providing {
        // should eventually include info about model
        peer: PeerId,
    },
}

impl MinMapHeapable<PeerId> for PendingTransaction {
    fn lookup_key(&self) -> PeerId {
        self.client
    }
}

impl<'w> NodeType<'w> for ProviderNode {
    fn wallet_val(&'w mut self) -> &'w mut Wallet {
        &mut self.wallet
    }
    async fn loop_logic(node: &mut Node<Self>) -> MainResult<()> {
        match node.typ.state {
            ProviderNodeState::Idle => match node.typ.pending_pool.pop().ok() {
                None => {
                    warn!("empty pending pool");
                }
                Some(tx) => {
                    node.typ.state = ProviderNodeState::Bidding(tx);
                    return Ok(());
                }
            },
            ProviderNodeState::Bidding(ref tx) => {
                warn!("bidding for {tx:#?}");
                let bid = gossip::ProvisionBid::new(node.swarm.local_peer_id().to_owned(), 50, 50.);

                let _ = node
                    .swarm
                    .behaviour_mut()
                    .gossip
                    .publish(
                        SysTopic::from(&tx.client).publish(),
                        serde_json::to_vec(&bid).expect("failed to serialize bid"),
                    )
                    .expect("failed to publish bid");
            }
            ProviderNodeState::Providing { peer } => {
                // This should eventually call a model and stream the tokens to the client
                let topic = SysTopic::from(&peer).publish();
                for i in 0..5 {
                    warn!("sending message");
                    node.swarm
                        .behaviour_mut()
                        .gossip
                        .publish(
                            topic.clone(),
                            serde_json::to_vec(&CompletionMessage::Working {
                                idx: i,
                                token: format!("message{i}"),
                            })
                            .expect("couldn't serialize message"),
                        )
                        .expect("failed to publish");
                }
                let local_id = *node.swarm.local_peer_id();
                node.swarm
                    .behaviour_mut()
                    .gossip
                    .publish(
                        topic,
                        serde_json::to_vec(&CompletionMessage::Finished {
                            peer: local_id,
                            total_messages: 5,
                        })
                        .expect("couldn't serialize message"),
                    )
                    .expect("failed to publish");
            }
        }
        tokio::select! {
            event = node.swarm.select_next_some() => Self::default_handle_swarm_event(node, event).await,
        }
    }

    fn init(swarm: &mut Swarm<SysBehaviour>) -> MainResult<Self> {
        swarm
            .behaviour_mut()
            .gossip
            .subscribe(&SysTopic::Pending.subscribe())?;

        // let key = RecordKey::new(&MODEL_ID_0);
        // swarm.behaviour_mut().kad.set_mode(Some(Mode::Server));

        // let _ = swarm
        //     .behaviour_mut()
        //     .kad
        //     .start_providing(key.clone())
        //     .expect("failed to make node a provider");

        Ok(ProviderNode {
            wallet: Wallet::new(),
            pending_pool: vec![].into(),
            state: ProviderNodeState::default(),
        })
    }

    async fn handle_swarm_event(
        node: &mut super::Node<Self>,
        event: libp2p::swarm::SwarmEvent<crate::behavior::SysBehaviourEvent>,
    ) -> MainResult<()> {
        match event {
            SwarmEvent::Behaviour(SysBehaviourEvent::ReqRes(
                request_response::Event::Message {
                    peer,
                    message:
                        request_response::Message::Request {
                            request, channel, ..
                        },
                },
            )) => {
                warn!("got request: {request:#?} from peer: {peer:#?}");
                node.swarm
                    .behaviour_mut()
                    .req_res
                    .send_response(channel, CompConnectConfirm { ok: true })
                    .expect("failed to send response");
                node.typ.state = ProviderNodeState::Providing { peer };
            }

            SwarmEvent::Behaviour(SysBehaviourEvent::Gossip(gossipsub::Event::Message {
                message: gossipsub::Message { data, topic, .. },
                ..
            })) if topic == SysTopic::Pending.publish() => {
                let tx: PendingTransaction = serde_json::from_slice(&data)?;
                warn!("provider received transaction: {tx:#?}");
                node.typ.pending_pool.insert(tx);
            }
            _ => {
                warn!("unhandled event by provider: {event:#?}")
            }
        }
        Ok(())
    }
}
