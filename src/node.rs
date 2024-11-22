use crate::{
    behavior::{CompletionReq, SysBehaviour, SysBehaviourEvent},
    chain::{block::Block, transaction::Transaction},
    CHAIN_TOPIC, MODEL_ID_0, TX_TOPIC,
};
use libp2p::{
    gossipsub::{self, Message, TopicHash},
    kad::{self, Addresses, GetProvidersOk, KBucketKey, QueryId, QueryResult},
    swarm::SwarmEvent,
    PeerId, Swarm,
};
use tokio::io::{AsyncBufReadExt, BufReader, Lines, Stdin};
use tracing::warn;

pub struct Node {
    // i hate that this is public
    pub typ: NodeType,
    ledger: Vec<Block>,
}

#[derive(Debug)]
pub enum NodeType {
    Client {
        stdin: Lines<BufReader<Stdin>>,
        user_input: Option<String>,
        provider_query_id: Option<QueryId>,
        // comp_topic: Option<IdentTopic>,
    },
    Validator {
        // vec might not be the best way to do this but it is fine for now
        tx_pool: Vec<Transaction>,
    },
    Provider,
}

impl NodeType {
    async fn try_read_stdin(&mut self) -> Option<String> {
        if let Self::Client { stdin, .. } = self {
            return stdin.next_line().await.unwrap();
        }
        None
    }
    pub fn is_validator(&self) -> bool {
        if let Self::Validator { .. } = self {
            return true;
        }
        false
    }
    pub fn is_client(&self) -> bool {
        if let Self::Client { .. } = self {
            return true;
        }
        false
    }
    pub fn is_provider(&self) -> bool {
        if let Self::Provider = self {
            return true;
        }
        false
    }
    pub fn new_client() -> Self {
        NodeType::Client {
            stdin: tokio::io::BufReader::new(tokio::io::stdin()).lines(),
            provider_query_id: None,
            user_input: None,
            // comp_topic: None,
        }
    }
}

impl Node {
    pub fn new(typ: NodeType, ledger: Vec<Block>) -> Node {
        Node { typ, ledger }
    }

    pub fn ledger_bytes(&self) -> serde_json::Result<Vec<u8>> {
        serde_json::to_vec(&self.ledger)
    }

    /// If other ledger is longer, replace mine with other.
    /// Returns whether or not ledger was replaced
    pub fn replace_ledger(&mut self, other: Vec<Block>) -> bool {
        let replace = other.len() > self.ledger.len();
        if replace {
            self.ledger = other;
        }

        replace
    }

    pub async fn handle_swarm_event(
        &mut self,
        swarm: &mut Swarm<SysBehaviour>,
        event: SwarmEvent<SysBehaviourEvent>,
    ) {
        match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                warn!("Listening on {address:?}");
                swarm.add_peer_address(*swarm.local_peer_id(), address);
                // swarm.behaviour_mut().kad.add_address(&self.id, address);
                // swarm.behaviour_mut().req_res.add_address(&self.id, address);
            }

            SwarmEvent::Behaviour(beh_event) => {
                match beh_event {
                    SysBehaviourEvent::Gossip(gossipsub::Event::Message {
                        message: Message { data, topic, .. },
                        ..
                    }) => {
                        match topic {
                            _ if topic == TopicHash::from_raw(CHAIN_TOPIC) => {
                                let update: Vec<Block> = serde_json::from_slice(&data)
                                    .expect("failed to deserialize chain update");
                                warn!("received chain update: {update:#?}");
                                if self.replace_ledger(update) {
                                    warn!("replaced self's ledger");
                                } else {
                                    warn!("did not replace self's ledger");
                                }
                            }

                            _ if topic == TopicHash::from_raw(TX_TOPIC)
                                && self.typ.is_validator() =>
                            {
                                // redundant check is smelly
                                if let NodeType::Validator { ref mut tx_pool } = &mut self.typ {
                                    warn!("validator received transaction");
                                    let tx: Transaction = serde_json::from_slice(&data)
                                        .expect("failed to deserialize chain update");
                                    tx_pool.push(tx);
                                }
                            }
                            _ => {}
                        }
                    }

                    SysBehaviourEvent::Kad(kad_e) => match kad_e {
                        kad::Event::OutboundQueryProgressed {
                            id, result, stats, ..
                        } => {
                            if let NodeType::Client {
                                provider_query_id,
                                user_input,
                                ..
                            } = &self.typ
                            {
                                if provider_query_id.is_some_and(|pid| pid == id) {
                                    if let QueryResult::GetProviders(res) = result {
                                        match res {
                                            Ok(ok) => {
                                                warn!("get providers ok: {ok:#?}");
                                                match ok {
                                                        GetProvidersOk::FoundProviders { key, providers } => {
                                                            let provider =
                                                                providers.into_iter().next().unwrap();
                                                            warn!("client is attempting to send request to server");
                                                            let local_id = swarm.local_peer_id().clone();
                                                            // swarm.behaviour_mut().req_res.send_request(
                                                            //     &provider,
                                                            //     CompletionReq::new(&local_id, &user_input.as_ref().unwrap(), MODEL_ID_0)
                                                            // );
                                                        }
                                                        GetProvidersOk::FinishedWithNoAdditionalRecord {
                                                            closest_peers,
                                                        } => {}
                                                    }
                                            }
                                            Err(err) => {
                                                warn!("get providers err: {err:#?}");
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        _ => {
                            warn!("unhandled kad event: {kad_e:#?}");
                        }
                    },
                    SysBehaviourEvent::ReqRes(event) => {
                        warn!("reqres event: {event:#?}");
                    }
                    _ => {
                        warn!("unhandled behavior event: {beh_event:#?}");
                    }
                }
            }
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                warn!("New connection to peer: {peer_id:#?}")
            }
            SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                warn!("Closed connection to peer: {peer_id:#?}\ncause: {cause:#?}")
            }
            event => {
                tracing::error!("Unhandled Event: {event:#?}");
            }
        }
    }
}
