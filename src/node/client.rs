use super::{Node, NodeType};
use crate::{
    behavior::{CompletionReq, SysBehaviour, SysBehaviourEvent},
    MainResult, MODEL_ID_0,
};
use futures::StreamExt;
use libp2p::{
    kad::{self, GetProvidersOk, QueryId, QueryResult, RecordKey},
    swarm::SwarmEvent,
    Swarm,
};
use tokio::io::{AsyncBufReadExt, BufReader, Lines, Stdin};
use tracing::warn;

#[derive(Debug)]
pub struct ClientNode {
    stdin: Lines<BufReader<Stdin>>,
    user_input: Option<String>,
    provider_query_id: Option<QueryId>,
}

impl NodeType for ClientNode {
    fn init(_swarm: &mut Swarm<SysBehaviour>) -> MainResult<Self> {
        Ok(ClientNode {
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
                    let all_ = node.swarm.connected_peers();
                    warn!("allpeers:");
                    for peer in all_{
                        warn!("PEER: {peer:#?}");
                    }

                    node.typ.user_input = Some(line);

                    let key = RecordKey::new(&MODEL_ID_0);
                     node.typ.provider_query_id = Some(node.swarm
                        .behaviour_mut()
                        .kad
                        .get_providers(key.clone()));
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
                                            let local_id = node.swarm.local_peer_id().clone();
                                            node.swarm.behaviour_mut().req_res.send_request(
                                                &provider,
                                                CompletionReq::new(
                                                    &local_id,
                                                    &node.typ.user_input.as_ref().unwrap(),
                                                    MODEL_ID_0,
                                                ),
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
            _ => {
                warn!("unhandled client event: {event:#?}");
            }
        }
        Ok(())
    }
}
