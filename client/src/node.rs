use crate::{
    behaviour::ClientNodeBehaviour,
    rpc::{ClientRequestWrapper, StartAuctionResponse},
};
use core::{
    behaviour::{
        gossip::NetworkTopic,
        req_res::{NetworkRequest, NetworkResponse},
        streaming::{connection_handler, StreamMessage},
        ProvisionBid,
    },
    node::{behaviour::NodeBehaviourEvent, Node, NodeType, NodeTypeEvent},
    util::{heap::max::MaxHeap, OneOf},
    MainResult,
};
use libp2p::{
    futures::StreamExt,
    gossipsub, request_response,
    swarm::{NetworkBehaviour, SwarmEvent},
    PeerId, Swarm,
};
use seraphic::{socket, RpcRequestWrapper};
use serde_json::json;
use std::time::Duration;
use tracing::warn;

#[derive(Debug)]
pub struct ClientNode {
    state: ClientNodeState,
}
type State = ClientNodeState;

#[derive(Debug)]
enum ClientNodeState {
    Idle,
    Auctioning {
        start: std::time::Instant,
        bids: MaxHeap<ProvisionBid>,
    },
    AttemptingConnection {
        bid: ProvisionBid,
        provider: PeerId,
    },
    DirectlyConnected {
        provider: PeerId,
        messages: Vec<StreamMessage>,
    },
}
const AUCTIONING_DURATION: Duration = Duration::from_millis(100);

impl ClientNode {
    fn start_auction(node: &mut Node<Self>) -> MainResult<()> {
        // definately should be a struct later, but for now this is fine
        let data = json!({
        // amt of input tokens
            "input_length": 50
        });

        node.swarm
            .behaviour_mut()
            .shared
            .gossip
            .publish(
                NetworkTopic::Auction.publish(),
                serde_json::to_vec(&data).unwrap(),
            )
            .expect("failed to publish auction start");
        node.inner.state = ClientNodeState::Auctioning {
            start: std::time::Instant::now(),
            bids: vec![].into(),
        };
        Ok(())
    }
}

#[derive(Debug)]
pub enum ClientNodeEvent {
    UserInput(String),
    ChoseBid(ProvisionBid),
    GotCompletion { provider: PeerId, content: String },
}

impl NodeTypeEvent for ClientNodeEvent {}

impl NodeType for ClientNode {
    type Behaviour = ClientNodeBehaviour;
    type Event = ClientNodeEvent;
    type RpcRequest = ClientRequestWrapper;

    fn init_with_swarm(_swarm: &mut Swarm<Self::Behaviour>) -> MainResult<Self>
    where
        Self: Sized,
    {
        let this_peer_id = _swarm.local_peer_id().clone();
        _swarm
            .behaviour_mut()
            .shared
            .gossip
            .subscribe(&NetworkTopic::from(&this_peer_id).subscribe())
            .expect("failed to subscribe to local topic");
        Ok(Self {
            state: ClientNodeState::Idle,
        })
    }

    async fn next_event(&mut self) -> MainResult<Option<Self::Event>> {
        match &mut self.state {
            ClientNodeState::Idle => Ok(None),
            ClientNodeState::Auctioning { start, bids } => {
                let elapsed = start.elapsed();
                tracing::warn!("elapsed: {elapsed:#?}");
                if elapsed >= AUCTIONING_DURATION && bids.len() > 0 {
                    if bids.peek().is_some() {
                        let bid = bids.pop().expect("failed to get bid from heap");
                        return Ok(Some(ClientNodeEvent::ChoseBid(bid)));
                    }
                }
                Ok(None)
            }
            ClientNodeState::DirectlyConnected { provider, messages } => Ok(None),
            ClientNodeState::AttemptingConnection { bid, provider } => Ok(None),
        }
    }

    async fn handle_self_event(node: &mut Node<Self>, e: Self::Event) -> MainResult<()>
    where
        Self: Sized,
    {
        tracing::warn!("client event: {e:#?}");
        match (e, &node.inner.state) {
            (ClientNodeEvent::ChoseBid(bid), ClientNodeState::Auctioning { .. }) => {
                node.swarm
                    .behaviour_mut()
                    .shared
                    .req_res
                    .send_request(&bid.peer, NetworkRequest::OpenStream);

                // maybe there is no need for attempting connection?
                node.inner.state = ClientNodeState::AttemptingConnection {
                    provider: bid.peer,
                    bid,
                }
                // if !node.swarm.is_connected(&bid.peer) {
                //     node.swarm
                //         .dial(bid.peer)
                //         .expect("client failed to dial peer");
                // }
                //
                // let control = node.swarm.behaviour_mut().shared.stream.new_control();
                //
                // node.swarm.dial
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_swarm_event(
        node: &mut Node<Self>,
        _e: SwarmEvent<<Self::Behaviour as NetworkBehaviour>::ToSwarm>,
    ) -> MainResult<Option<SwarmEvent<<Self::Behaviour as NetworkBehaviour>::ToSwarm>>>
    where
        Self: Sized,
    {
        match (_e, &mut node.inner.state) {
            (
                SwarmEvent::NewListenAddr { address, .. },
                State::DirectlyConnected { provider, messages },
            ) => {
                let listen_address = address.with_p2p(*node.swarm.local_peer_id()).unwrap();
                tracing::info!(%listen_address);
                Ok(None)
            }
            (
                SwarmEvent::Behaviour(NodeBehaviourEvent::Gossip(gossipsub::Event::Message {
                    message: gossipsub::Message { topic, data, .. },
                    ..
                })),
                State::Auctioning { ref mut bids, .. },
            ) if topic == NetworkTopic::from(node.swarm.local_peer_id()).publish() => {
                let bid: ProvisionBid =
                    serde_json::from_slice(&data).expect("failed to serialize bid data");
                tracing::warn!("received bid: {bid:#?}");
                bids.insert(bid);
                Ok(None)
            }
            (
                SwarmEvent::Behaviour(NodeBehaviourEvent::ReqRes(
                    request_response::Event::Message {
                        peer,
                        message:
                            request_response::Message::Response {
                                request_id,
                                response: NetworkResponse::OpenStreamAck { opened },
                            },
                    },
                )),
                State::AttemptingConnection { bid, provider },
            ) => {
                if !opened {
                    tracing::error!("provider is busy, could not connect. Returning to idle state");
                    node.inner.state = State::Idle;
                    return Ok(None);
                }

                if !node.swarm.is_connected(provider) {
                    node.swarm.dial(*provider)?;
                }

                tokio::spawn(connection_handler(
                    *provider,
                    node.swarm.behaviour().shared.stream.new_control(),
                ));

                node.inner.state = State::DirectlyConnected {
                    provider: *provider,
                    messages: vec![],
                };
                Ok(None)
            }
            (event, _state) => Ok(Some(event)),
        }
    }

    async fn handle_rpc_request(
        _node: &mut Node<Self>,
        _r: socket::Request,
    ) -> MainResult<core::util::OneOf<socket::Request, seraphic::ProcessRequestResult>>
    where
        Self: Sized,
    {
        let req = match Self::RpcRequest::try_from_rpc_req(_r.clone()) {
            Err(_) => return Ok(OneOf::Left(_r)),
            Ok(req) => req,
        };

        match req {
            ClientRequestWrapper::StartAuction(req) => {
                warn!("client handling StartAuction");
                let start = ClientNode::start_auction(_node);
                let response = StartAuctionResponse {
                    started: start.is_ok(),
                };
                let json = serde_json::to_value(response)?;
                return Ok(OneOf::Right(Ok(json)));
            }
        }
    }
}
