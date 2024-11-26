use super::NodeType;
use crate::{
    behavior::{SubResponse, SysBehaviour, SysBehaviourEvent},
    chain::transaction::PendingTransaction,
    node::client::CompletionMessage,
    MainResult, MODEL_ID_0,
};
use libp2p::{
    gossipsub::{self},
    kad::{Mode, RecordKey},
    request_response, Swarm,
};
use tracing::warn;

#[derive(Debug)]
pub struct ProviderNode {
    pending_pool: Vec<PendingTransaction>,
}

impl NodeType for ProviderNode {
    fn init(swarm: &mut Swarm<SysBehaviour>) -> MainResult<Self> {
        // this could be causing bug!
        swarm.behaviour_mut().kad.set_mode(Some(Mode::Server));
        // completely arbitrary rn, not quite sure how to implement model providing yet
        let key = RecordKey::new(&MODEL_ID_0);

        let _ = swarm
            .behaviour_mut()
            .kad
            .start_providing(key.clone())
            .expect("failed to make node a provider");

        Ok(ProviderNode)
    }

    async fn handle_swarm_event(
        node: &mut super::Node<Self>,
        event: libp2p::swarm::SwarmEvent<crate::behavior::SysBehaviourEvent>,
    ) -> MainResult<()> {
        match event {
            libp2p::swarm::SwarmEvent::Behaviour(SysBehaviourEvent::ReqRes(
                request_response::Event::Message {
                    peer,
                    message:
                        request_response::Message::Request {
                            request, channel, ..
                        },
                },
            )) => {
                warn!("got request: {request:#?} from peer: {peer:#?}");
                let topic = gossipsub::IdentTopic::new(request.topic);
                // let subscribe_error = match node.swarm.behaviour_mut().gossip.subscribe(&topic) {
                //     Ok(_) => None,
                //     Err(e) => Some(e.to_string()),
                // };

                node.swarm
                    .behaviour_mut()
                    .req_res
                    .send_response(
                        channel,
                        SubResponse {
                            subscribe_error: None,
                        },
                    )
                    .expect("failed to send response");

                // This should eventually call a model and stream the tokens to the client
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
                        topic.clone(),
                        serde_json::to_vec(&CompletionMessage::Finished {
                            peer: local_id,
                            total_messages: 5,
                        })
                        .expect("couldn't serialize message"),
                    )
                    .expect("failed to publish");

                // node.swarm
                //     .behaviour_mut()
                //     .gossip
                //     .unsubscribe(&topic)
                //     .expect("failed to unsub");
            }
            _ => {
                warn!("unhandled event by provider: {event:#?}")
            }
        }
        Ok(())
    }
}
