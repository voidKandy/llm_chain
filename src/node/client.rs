use super::*;
use crate::behaviour_util::{NetworkTopic, ProvisionBid};
use crate::heap::max::MaxHeap;
use crate::MainResult;
use behaviour::{ClientNodeBehaviour, NodeBehaviourEvent};
use libp2p::{gossipsub, PeerId};
use serde_json::json;
use tokio::io::{AsyncBufReadExt, BufReader, Lines, Stdin};

const AUCTIONING_DURATION: Duration = Duration::from_millis(100);
pub struct ClientNode {
    state: ClientNodeState,
}

pub enum ClientNodeState {
    Idle {
        stdin: Lines<BufReader<Stdin>>,
    },
    Auctioning {
        start: std::time::Instant,
        bids: MaxHeap<ProvisionBid>,
    },
    AttemptingConnection {
        sent_request: bool,
        bid: ProvisionBid,
        provider: PeerId,
    },
    GettingCompletion {
        provider: PeerId,
        expected_amt_messages: Option<usize>,
        messages: Vec<(usize, String)>,
    },
}

impl Node<ClientNode> {
    fn start_auction(&mut self) -> MainResult<()> {
        // definately should be a struct later, but for now this is fine
        let data = json!({
        // amt of input tokens
        "input_length": 50
        });

        self.swarm
            .behaviour_mut()
            .shared
            .gossip
            .publish(
                NetworkTopic::Auction.publish(),
                serde_json::to_vec(&data).unwrap(),
            )
            .expect("failed to publish auction start");
        self.inner.state = ClientNodeState::Auctioning {
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
            state: ClientNodeState::Idle {
                stdin: tokio::io::BufReader::new(tokio::io::stdin()).lines(),
            },
        })
    }

    async fn next_event(&mut self) -> MainResult<Option<Self::Event>> {
        match &mut self.state {
            ClientNodeState::Idle { stdin } => Ok(stdin
                .next_line()
                .await?
                .and_then(|l| Some(ClientNodeEvent::UserInput(l)))),
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
            ClientNodeState::AttemptingConnection {
                sent_request,
                bid,
                provider,
            } => Ok(None),
        }
    }

    async fn handle_self_event(node: &mut Node<Self>, e: Self::Event) -> MainResult<()>
    where
        Self: Sized,
    {
        tracing::warn!("client event: {e:#?}");
        Ok(())
    }

    async fn handle_swarm_event(
        node: &mut Node<Self>,
        _e: SwarmEvent<<Self::Behaviour as NetworkBehaviour>::ToSwarm>,
    ) -> MainResult<Option<SwarmEvent<<Self::Behaviour as NetworkBehaviour>::ToSwarm>>>
    where
        Self: Sized,
    {
        if let (
            SwarmEvent::Behaviour(NodeBehaviourEvent::Gossip(gossipsub::Event::Message {
                message: gossipsub::Message { topic, data, .. },
                ..
            })),
            ClientNodeState::Auctioning { ref mut bids, .. },
        ) = (&_e, &mut node.inner.state)
        {
            let this_peer_id = node.swarm.local_peer_id().clone();
            if *topic == NetworkTopic::from(&this_peer_id).publish() {
                let bid: ProvisionBid =
                    serde_json::from_slice(&data).expect("failed to serialize bid data");
                tracing::warn!("received bid: {bid:#?}");
                bids.insert(bid);
                return Ok(None);
            }
        }

        return Ok(Some(_e));
    }
}
