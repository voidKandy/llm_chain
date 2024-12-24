use behaviour::NodeBehaviourEvent;
use core::{
    node::*,
    util::behaviour::{
        gossip::NetworkTopic,
        req_res::NetworkResponse,
        streaming::{echo, STREAM_PROTOCOL},
    },
    MainResult,
};
use libp2p::{futures::StreamExt, gossipsub, request_response, swarm::SwarmEvent, Swarm};
use rpc::RequestWrapper;
use tokio::task::JoinHandle;

use crate::behaviour::ServerNodeBehaviour;

/// provides model work

#[derive(Debug)]
pub struct ProviderNode {
    state: ProviderNodeState,
}
type State = ProviderNodeState;
#[derive(Debug)]
enum ProviderNodeState {
    Idle,
    ListeningForStream(JoinHandle<()>),
}

#[derive(Debug)]
pub enum ProviderNodeEvent {}
impl NodeTypeEvent for ProviderNodeEvent {}

impl ProviderNode {
    fn start_listening_for_stream(node: &mut Node<Self>) -> MainResult<()> {
        let mut incoming_streams = node
            .swarm
            .behaviour_mut()
            .shared
            .stream
            .new_control()
            .accept(STREAM_PROTOCOL)
            .unwrap();

        let handle = tokio::spawn(async move {
            // This loop handles incoming streams _sequentially_ but that doesn't have to be the case.
            // You can also spawn a dedicated task per stream if you want to.
            // Be aware that this breaks backpressure though as spawning new tasks is equivalent to an
            // unbounded buffer. Each task needs memory meaning an aggressive remote peer may
            // force you OOM this way.

            while let Some((peer, stream)) = incoming_streams.next().await {
                match echo(stream).await {
                    Ok(n) => {
                        tracing::info!(%peer, "Echoed {n} bytes!");
                    }
                    Err(e) => {
                        tracing::warn!(%peer, "Echo failed: {e}");
                        continue;
                    }
                };
            }
        });
        let state = State::ListeningForStream(handle);
        node.inner.state = state;
        Ok(())
    }
}

impl NodeType for ProviderNode {
    type Behaviour = ServerNodeBehaviour;
    type Event = ProviderNodeEvent;
    type RpcRequest = RequestWrapper;

    fn init_with_swarm(swarm: &mut Swarm<Self::Behaviour>) -> MainResult<Self>
    where
        Self: Sized,
    {
        swarm
            .behaviour_mut()
            .shared
            .gossip
            .subscribe(&NetworkTopic::Auction.subscribe())
            .expect("failed to sub to auction topic");

        Ok(Self { state: State::Idle })
    }

    async fn next_event(&mut self) -> MainResult<Option<Self::Event>> {
        // tokio::select! {
        // swarm_event = self.swarm.select_next_some() => {
        //     Ok(Some(NodeEvent::from(swarm_event)))
        // }
        // }
        //
        Ok(None)
    }
    async fn handle_self_event(node: &mut Node<Self>, e: Self::Event) -> MainResult<()>
    where
        Self: Sized,
    {
        tracing::warn!("server event: {e:#?}");
        Ok(())
    }

    async fn handle_swarm_event(
        node: &mut Node<Self>,
        _e: libp2p::swarm::SwarmEvent<
            <Self::Behaviour as libp2p::swarm::NetworkBehaviour>::ToSwarm,
        >,
    ) -> MainResult<
        Option<
            libp2p::swarm::SwarmEvent<
                <Self::Behaviour as libp2p::swarm::NetworkBehaviour>::ToSwarm,
            >,
        >,
    >
    where
        Self: Sized,
    {
        match (_e, &node.inner.state) {
            (
                SwarmEvent::Behaviour(NodeBehaviourEvent::ReqRes(
                    libp2p::request_response::Event::Message {
                        peer,
                        message:
                            request_response::Message::Request {
                                request_id,
                                request,
                                channel,
                            },
                    },
                )),
                State::Idle,
            ) => {
                Self::start_listening_for_stream(node);
                node.swarm
                    .behaviour_mut()
                    .shared
                    .req_res
                    .send_response(channel, NetworkResponse::OpenStreamAck)
                    .unwrap();
                Ok(None)
            }
            (
                SwarmEvent::Behaviour(NodeBehaviourEvent::Gossip(
                    libp2p::gossipsub::Event::Message {
                        message:
                            gossipsub::Message {
                                topic,
                                data,
                                source,
                                ..
                            },
                        ..
                    },
                )),
                State::Idle,
            ) if topic == NetworkTopic::Auction.publish() => {
                // should send a bid
                Ok(None)
            }
            (event, _state) => return Ok(Some(event)),
        }
    }
}
