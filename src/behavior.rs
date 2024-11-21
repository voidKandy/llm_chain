use libp2p::{
    gossipsub::{self, MessageAuthenticity, TopicHash},
    identify,
    identity::Keypair,
    kad::{self, store::MemoryStore, GetProvidersOk, QueryResult},
    swarm::{NetworkBehaviour, SwarmEvent},
    PeerId, StreamProtocol, Swarm,
};
use serde::{Deserialize, Serialize};
use std::hash::{DefaultHasher, Hash, Hasher};
use tracing::warn;

use crate::{
    chain::{block::Block, transaction::Transaction},
    node::{Node, NodeType},
    CHAIN_TOPIC, COMPLETION_TOPIC, MODEL_ID_0, TX_TOPIC,
};

#[derive(Debug, Deserialize, Serialize)]
pub struct CompletionReq<'m> {
    pub model_id: &'m str,
    pub prompt: &'m str,
}

const IDENTIFY_ID: &str = "/ipfs/id/1.0.0";
#[derive(NetworkBehaviour)]
pub struct SysBehaviour {
    pub gossip: gossipsub::Behaviour,
    pub kad: kad::Behaviour<MemoryStore>,
    pub identify: identify::Behaviour,
}

impl SysBehaviour {
    pub fn new(peer_id: PeerId, key: Keypair) -> Self {
        let message_id_fn = |message: &gossipsub::Message| {
            let mut s = DefaultHasher::new();
            message.data.hash(&mut s);
            gossipsub::MessageId::from(s.finish().to_string())
        };
        let identify =
            identify::Behaviour::new(identify::Config::new(IDENTIFY_ID.to_string(), key.public()));

        let peer_store = MemoryStore::new(peer_id);
        let gossip_config = gossipsub::ConfigBuilder::default()
            .message_id_fn(message_id_fn)
            .build()
            .expect("failed to build gossip config");

        let gossip =
            gossipsub::Behaviour::new(MessageAuthenticity::Signed(key), gossip_config).unwrap();

        // fairly certain this protocol name is arbitrary
        let kad_config = kad::Config::new(StreamProtocol::new("/main"));

        // Good for debugging, by default this is set to 5 mins
        // kad_config.set_periodic_bootstrap_interval(Some(Duration::from_secs(10)));

        let kad = kad::Behaviour::<MemoryStore>::with_config(peer_id, peer_store, kad_config);

        SysBehaviour {
            gossip,
            kad,
            identify,
        }
    }
}

pub async fn handle_swarm_event(
    node: &mut Node,
    swarm: &mut Swarm<SysBehaviour>,
    event: SwarmEvent<SysBehaviourEvent>,
) {
    match event {
        SwarmEvent::NewListenAddr { address, .. } => warn!("Listening on {address:?}"),
        SwarmEvent::Behaviour(beh_event) => {
            match beh_event {
                SysBehaviourEvent::Gossip(gossipsub::Event::Message {
                    propagation_source,
                    message_id,
                    message,
                }) => {
                    match message.topic {
                        _ if message.topic == TopicHash::from_raw(CHAIN_TOPIC) => {
                            let update: Vec<Block> = serde_json::from_slice(&message.data)
                                .expect("failed to deserialize chain update");
                            warn!("received chain update: {update:#?}");
                            if node.replace_ledger(update) {
                                warn!("replaced node's ledger");
                            } else {
                                warn!("did not replace node's ledger");
                            }
                        }
                        _ if message.topic == TopicHash::from_raw(COMPLETION_TOPIC)
                            && node.typ.is_provider() =>
                        {
                            warn!("received message for completion");
                            let deserialized: CompletionReq = serde_json::from_slice(&message.data)
                                .expect("failed to deserialize");
                            warn!("Got message: '{deserialized:#?}' with id: {message_id} from peer: {propagation_source}");
                        }
                        _ if message.topic == TopicHash::from_raw(TX_TOPIC)
                            && node.typ.is_validator() =>
                        {
                            // redundant check is smelly
                            if let NodeType::Validator { ref mut tx_pool } = &mut node.typ {
                                warn!("validator received transaction");
                                let tx: Transaction = serde_json::from_slice(&message.data)
                                    .expect("failed to deserialize chain update");
                                tx_pool.push(tx);
                            }
                        }
                        _ => {}
                    }
                }

                SysBehaviourEvent::Kad(kad_e) => match kad_e {
                    kad::Event::OutboundQueryProgressed {
                        id,
                        result,
                        stats,
                        step,
                    } => {
                        // if client_node_data.provider_query == Some(id) {
                        //     warn!("client node found a provider:\nresult={result:#?}\nstats={stats:#?}");
                        // }
                        match result {
                            QueryResult::StartProviding(res) => {
                                warn!("started providing: {res:#?}");
                            }
                            QueryResult::GetProviders(res) => {
                                let res = res.expect("failed to unwrap get providers res");
                                warn!("get providers result: {res:#?}");
                                match res {
                                    GetProvidersOk::FoundProviders { key, providers } => {
                                        // let provider = providers.into_iter().next().unwrap();
                                    }
                                    GetProvidersOk::FinishedWithNoAdditionalRecord {
                                        closest_peers,
                                    } => {}
                                }
                            }
                            _ => {}
                        }
                    }

                    _ => {
                        warn!("kad event: {kad_e:#?}");
                    }
                },
                _ => {
                    warn!("unhandled behavior event: {beh_event:#?}");
                }
            }
        }
        SwarmEvent::ConnectionEstablished { peer_id, .. } => {
            warn!("New connection to peer: {peer_id:#?}")
        }
        SwarmEvent::ConnectionClosed { peer_id, .. } => {
            warn!("Closed connection to peer: {peer_id:#?}")
        }
        event => {
            tracing::error!("Unhandled Event: {event:#?}");
        }
    }
}
