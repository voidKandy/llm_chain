use super::{Node, NodeType};
use crate::{
    behavior::{SubRequest, SysBehaviour, SysBehaviourEvent},
    chain::transaction::Transaction,
    MainResult, MODEL_ID_0, TX_TOPIC,
};
use futures::StreamExt;
use libp2p::{
    gossipsub::{self, TopicHash},
    kad::{self, GetProvidersOk, QueryId, QueryResult, RecordKey},
    request_response,
    swarm::SwarmEvent,
    PeerId, Swarm,
};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader, Lines, Stdin};
use tracing::warn;

#[derive(Debug)]
pub struct ClientNode {
    /// TopicHash unique to this node
    my_topic: TopicHash,
    stdin: Lines<BufReader<Stdin>>,
    user_input: Option<String>,
    provider_query_id: Option<QueryId>,
    current_tx: Option<ClientTransactionInfo>,
}

#[derive(Debug)]
struct ClientTransactionInfo {
    tx: Transaction,
    expected_amt_messages: Option<usize>,
    messages: Vec<(usize, String)>,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum CompletionMessage {
    Working { idx: usize, token: String },
    Finished { peer: PeerId, total_messages: usize },
}

impl NodeType for ClientNode {
    fn init(swarm: &mut Swarm<SysBehaviour>) -> MainResult<Self> {
        let tx_topic = gossipsub::IdentTopic::new(TX_TOPIC);
        swarm.behaviour_mut().gossip.subscribe(&tx_topic)?;
        let my_topic = TopicHash::from_raw(swarm.local_peer_id().to_string());

        let topic = gossipsub::IdentTopic::new(my_topic.to_string());
        swarm
            .behaviour_mut()
            .gossip
            .subscribe(&topic)
            .expect("client failed to subscribe to it's unique topic'");

        Ok(ClientNode {
            my_topic,
            current_tx: None,
            stdin: tokio::io::BufReader::new(tokio::io::stdin()).lines(),
            provider_query_id: None,
            user_input: None,
        })
    }

    async fn tokio_select_branches(node: &mut Node<Self>) -> MainResult<()> {
        tokio::select! {
            event = node.swarm.select_next_some() => Self::default_handle_swarm_event(node, event).await,
            Ok(Some(line)) = node.typ.stdin.next_line() => {
                if node.typ.provider_query_id.is_none() {
                    // let all_ = node.swarm.connected_peers();
                    // warn!("allpeers:");
                    // for peer in all_{
                    //     warn!("PEER: {peer:#?}");
                    // }

                    node.typ.user_input = Some(line);

                    let key = RecordKey::new(&MODEL_ID_0);
                     // node.typ.provider_query_id = Some(node.swarm
                     //    .behaviour_mut()
                     //    .kad
                     //    .get_providers(key.clone()));

                    let tx_topic = gossipsub::IdentTopic::new(TX_TOPIC);
            let local_id = node.swarm.local_peer_id();
                    let data = serde_json::to_vec(&Transaction::new(*local_id, *local_id, 2.)).unwrap();
                    node.swarm.behaviour_mut().gossip.publish(tx_topic, data).unwrap();
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
                let message: CompletionMessage = serde_json::from_slice(&data).unwrap();
                warn!("client received: {message:#?}");

                match message {
                    CompletionMessage::Finished {
                        peer,
                        total_messages,
                    } => {
                        // should publish a transaction
                        node.typ.provider_query_id = None;
                        node.typ.user_input = None;
                        let current_tx = node.typ.current_tx.as_mut().unwrap();
                        assert_eq!(
                            current_tx.tx.provider, peer,
                            "somehow completion was signed with wrong signature"
                        );
                        current_tx.expected_amt_messages = Some(total_messages);
                    }
                    CompletionMessage::Working { idx, token } => {
                        let current_tx = node.typ.current_tx.as_mut().expect("tx should be some");
                        current_tx.messages.push((idx, token));
                    }
                }

                if let Some(tx) = node.typ.current_tx.as_ref() {
                    if let Some(exp) = tx.expected_amt_messages {
                        if exp == tx.messages.len() {
                            let tx_topic = gossipsub::IdentTopic::new(TX_TOPIC);
                            warn!("should publish tx: {:#?}", tx.tx);
                            node.swarm
                                .behaviour_mut()
                                .gossip
                                .publish(tx_topic, serde_json::to_vec(&tx.tx).unwrap())
                                .expect("failed publish");
                        }
                    }
                }
            }

            SwarmEvent::Behaviour(SysBehaviourEvent::Kad(kad_event)) => match kad_event {
                kad::Event::OutboundQueryProgressed { id, result, .. } => {
                    if node.typ.provider_query_id.is_some_and(|pid| pid == id) {
                        match result {
                            QueryResult::GetProviders(res) => match res {
                                Ok(ok) => {
                                    warn!("get providers ok: {ok:#?}");
                                    match ok {
                                        GetProvidersOk::FoundProviders { providers, .. } => {
                                            let provider = providers.into_iter().next().unwrap();
                                            warn!("client is attempting to send request to server");
                                            node.swarm.behaviour_mut().req_res.send_request(
                                                &provider,
                                                SubRequest {
                                                    topic: node.typ.my_topic.to_string(),
                                                },
                                            );
                                        }
                                        GetProvidersOk::FinishedWithNoAdditionalRecord {
                                            ..
                                        } => {}
                                    }
                                }
                                Err(err) => {
                                    warn!("get providers err: {err:#?}");
                                }
                            },
                            _ => {
                                warn!("unhandled kad query result: {result:#?}");
                            }
                        }
                    }
                }
                _ => {
                    warn!("unhandled kad event: {kad_event:#?}");
                }
            },
            SwarmEvent::Behaviour(SysBehaviourEvent::ReqRes(
                request_response::Event::Message {
                    peer,
                    message:
                        request_response::Message::Response {
                            request_id,
                            response,
                        },
                },
            )) => match response.subscribe_error {
                None => {
                    warn!("no error, ready to receive");
                    assert!(node.typ.current_tx.is_none(), "current tx cannot be some");
                    // 2.0 token val should change
                    node.typ.current_tx = Some(ClientTransactionInfo {
                        tx: Transaction::new(*node.swarm.local_peer_id(), peer, 2.0),
                        messages: vec![],
                        expected_amt_messages: None,
                    });
                }
                Some(err_str) => {
                    warn!("provider encountered an error when subbing: {err_str:#?}");
                }
            },
            _ => {
                warn!("unhandled client event: {event:#?}");
            }
        }
        Ok(())
    }
}
