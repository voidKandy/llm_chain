pub mod behaviour;
pub mod client;
pub mod provider;
pub mod validator;
use crate::{
    behaviour_util::{NetworkRequest, NetworkResponse},
    chain::{block::Blockchain, transaction::Transaction},
    MainResult,
};
use behaviour::{NodeBehaviourEvent, NodeNetworkBehaviour};
use futures::StreamExt;
use libp2p::{
    identity::{Keypair, SigningError},
    swarm::{NetworkBehaviour, Swarm, SwarmEvent},
    PeerId,
};
use sha3::{Digest, Sha3_256};
use std::{hash::DefaultHasher, str::FromStr, time::Duration};

pub struct Node<T: NodeType> {
    keys: Keypair,
    blockchain: Blockchain,
    pub swarm: Swarm<T::Behaviour>,
    pub inner: T,
}

/// Eventually will be a list of addrs/ids
pub const BOOT_NODE_PEER_ID: &str = "12D3KooWCwDGQ5jED2DCkdjLpfitvBr6KMDW3VkFLMxE4f67vUen";
pub const BOOT_NODE_LOCAL_ADDR: &str = "/ip4/127.0.0.1/udp/62649/quic-v1";
pub const BOOT_NODE_LISTEN_ADDR: &str = "/ip4/0.0.0.0/udp/62649/quic-v1";

impl<T> Node<T>
where
    T: NodeType,
    <<T as NodeType>::Behaviour as NetworkBehaviour>::ToSwarm: std::fmt::Debug,
    SwarmEvent<<<T as NodeType>::Behaviour as NetworkBehaviour>::ToSwarm>:
        Into<SwarmEvent<NodeBehaviourEvent>>,
{
    pub fn try_from_keys(keys: Keypair) -> MainResult<Self> {
        let mut swarm = Self::swarm(keys.clone())?;
        let inner = T::init_with_swarm(&mut swarm)?;
        let blockchain = Blockchain::new();
        Ok(Self {
            inner,
            swarm,
            blockchain,
            keys,
        })
    }

    pub async fn main_loop(&mut self) -> MainResult<()> {
        loop {
            tokio::select! {
                swarm_event = self.swarm.select_next_some() => {
                    tracing::warn!("swarm event: {swarm_event:#?}");
                    if let Some(event) = T::handle_swarm_event(self, swarm_event).await? {
                        self.handle_swarm_event(event).await?
                    }
                }
                Ok(Some(inner_event)) = self.inner.next_event() => {
                    tracing::warn!("inner event: {inner_event:#?}");
                    T::handle_self_event(self, inner_event).await?;
                }
            }
        }
    }

    /// This is where default swarm event handling should be implemented
    async fn handle_swarm_event(
        &mut self,
        event: impl Into<SwarmEvent<NodeBehaviourEvent>>,
    ) -> MainResult<()> {
        match Into::<SwarmEvent<NodeBehaviourEvent>>::into(event) {
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                T::Behaviour::shared(self.swarm.behaviour_mut())
                    .req_res
                    .send_request(&peer_id, NetworkRequest::Chain);
            }

            SwarmEvent::Behaviour(NodeBehaviourEvent::ReqRes(
                libp2p::request_response::Event::Message { message, .. },
            )) => match message {
                libp2p::request_response::Message::Request {
                    request, channel, ..
                } => match request {
                    NetworkRequest::Chain => {
                        T::Behaviour::shared(self.swarm.behaviour_mut())
                            .req_res
                            .send_response(channel, NetworkResponse::Chain(self.blockchain.clone()))
                            .expect("failed to send response");
                    }
                },

                libp2p::request_response::Message::Response { response, .. } => match response {
                    NetworkResponse::Chain(bc) => {
                        if bc.validate() {
                            tracing::warn!("chain valid, updating");
                            self.blockchain = bc;
                        } else {
                            tracing::warn!("chain invalid");
                        }
                    }
                },
            },

            _ => {}
        }
        Ok(())
    }

    fn swarm(keys: Keypair) -> MainResult<Swarm<T::Behaviour>> {
        Ok(libp2p::SwarmBuilder::with_existing_identity(keys)
            .with_tokio()
            .with_quic()
            .with_dns()?
            .with_behaviour(|key| T::Behaviour::new(key.clone()))?
            .with_swarm_config(|cfg| {
                cfg.with_idle_connection_timeout(Duration::from_secs(u64::MAX))
            })
            .build())
    }
}

pub trait NodeTypeEvent: std::fmt::Debug {}

#[allow(async_fn_in_trait, private_bounds)]
pub trait NodeType {
    type Behaviour: NodeNetworkBehaviour;
    type Event: NodeTypeEvent;
    /// Where any logic particular to the initialization of a swarm can be implemented
    /// (Particular gossip topics, etc..)
    fn init_with_swarm(swarm: &mut Swarm<Self::Behaviour>) -> MainResult<Self>
    where
        Self: Sized;

    async fn next_event(&mut self) -> MainResult<Option<Self::Event>>;
    async fn handle_self_event(node: &mut Node<Self>, e: Self::Event) -> MainResult<()>
    where
        Self: Sized;

    /// Allows individual nodes to override default event handling.
    /// Either consumes the event or returns it to be handled by the outer `Node<T>`'s default handling
    /// The type bounds is the same as `SwarmEvent<NodeBehaviourEvent>`, it just needs to be
    /// written this way to appease the compiler
    async fn handle_swarm_event(
        node: &mut Node<Self>,
        _e: SwarmEvent<<Self::Behaviour as NetworkBehaviour>::ToSwarm>,
    ) -> MainResult<Option<SwarmEvent<<Self::Behaviour as NetworkBehaviour>::ToSwarm>>>
    where
        Self: Sized,
    {
        Ok(Some(_e))
    }
}
