use crate::behaviour::ClientNodeBehaviour;
use core::{
    node::{behaviour::NodeBehaviourEvent, rpc::RequestWrapper, Node, NodeType, NodeTypeEvent},
    util::{
        behaviour::{
            gossip::NetworkTopic, req_res::NetworkRequest, streaming::connection_handler,
            ProvisionBid,
        },
        heap::max::MaxHeap,
        json_rpc::RpcHandler,
    },
    MainResult,
};
use libp2p::{
    futures::StreamExt,
    gossipsub, request_response,
    swarm::{NetworkBehaviour, SwarmEvent},
    PeerId, Swarm,
};
use serde_json::json;
use std::time::Duration;

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
    GettingCompletion {
        provider: PeerId,
        expected_amt_messages: Option<usize>,
        messages: Vec<(usize, String)>,
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
// MACRO COULD BE HERE
pub enum ClientNodeEvent {
    UserInput(String),
    ChoseBid(ProvisionBid),
    GotCompletion { provider: PeerId, content: String },
}
impl NodeTypeEvent for ClientNodeEvent {}

impl NodeType for ClientNode {
    type Behaviour = ClientNodeBehaviour;
    type Event = ClientNodeEvent;
    type RpcRequest = RequestWrapper;

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
                        // tracing::warn!("bid: {}, balance: {}", bid.bid, self.wallet.balance,);
                        // if self.wallet.balance > bid.bid {
                        return Ok(Some(ClientNodeEvent::ChoseBid(bid)));
                        // }
                    }
                }
                Ok(None)
            }
            ClientNodeState::GettingCompletion {
                provider,
                expected_amt_messages,
                messages,
            } => Ok(None),
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
                return Ok(None);
            }
            (
                SwarmEvent::Behaviour(NodeBehaviourEvent::ReqRes(
                    request_response::Event::Message {
                        peer,
                        message:
                            request_response::Message::Response {
                                request_id,
                                response,
                            },
                    },
                )),
                State::AttemptingConnection { bid, provider },
            ) => {
                // In this demo application, the dialing peer initiates the protocol.

                if !node.swarm.is_connected(provider) {
                    node.swarm.dial(*provider)?;
                }

                tokio::spawn(connection_handler(
                    *provider,
                    node.swarm.behaviour().shared.stream.new_control(),
                ));

                // Poll the swarm to make progress.
                loop {
                    let event = node.swarm.next().await.expect("never terminates");

                    match event {
                        libp2p::swarm::SwarmEvent::NewListenAddr { address, .. } => {
                            let listen_address =
                                address.with_p2p(*node.swarm.local_peer_id()).unwrap();
                            tracing::info!(%listen_address);
                        }
                        event => tracing::trace!(?event),
                    }
                }
            }
            (event, _state) => Ok(Some(event)),
        }
    }
}
