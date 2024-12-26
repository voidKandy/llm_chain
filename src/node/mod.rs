pub mod behaviour;
pub mod rpc;
use crate::{
    behaviour::gossip::NetworkTopic,
    blockchain::chain::{init_blockchain, Blockchain},
    util::OneOf,
    MainResult,
};
use behaviour::{NodeBehaviourEvent, NodeNetworkBehaviour};
use futures::StreamExt;
use libp2p::{
    gossipsub,
    identity::Keypair,
    swarm::{NetworkBehaviour, Swarm, SwarmEvent},
};
use seraphic::{
    socket::{self},
    thread::RpcListeningThread,
    ProcessRequestResult, RpcHandler, RpcRequestWrapper,
};
use std::{fmt::Debug, time::Duration};
use tokio::net::ToSocketAddrs;

pub struct Node<T: NodeType> {
    keys: Keypair,
    rpc_thread: RpcListeningThread,
    blockchain: Blockchain,
    pub swarm: Swarm<T::Behaviour>,
    pub inner: T,
}

impl<T> AsMut<Node<T>> for Node<T>
where
    T: NodeType,
    <<T as NodeType>::Behaviour as NetworkBehaviour>::ToSwarm: std::fmt::Debug,
    SwarmEvent<<<T as NodeType>::Behaviour as NetworkBehaviour>::ToSwarm>:
        Into<SwarmEvent<NodeBehaviourEvent>>,
{
    fn as_mut(&mut self) -> &mut Node<T> {
        self
    }
}

impl<T> Node<T>
where
    T: NodeType,
    <<T as NodeType>::Behaviour as NetworkBehaviour>::ToSwarm: std::fmt::Debug,
    SwarmEvent<<<T as NodeType>::Behaviour as NetworkBehaviour>::ToSwarm>:
        Into<SwarmEvent<NodeBehaviourEvent>>,
{
    pub async fn try_from_keys(keys: Keypair, addr: impl ToSocketAddrs) -> MainResult<Self> {
        let mut swarm = Self::swarm(keys.clone())?;
        let inner = T::init_with_swarm(&mut swarm)?;
        let blockchain = init_blockchain();
        Ok(Self {
            inner,
            swarm,
            blockchain,
            keys,
            rpc_thread: RpcListeningThread::new(addr).await?,
        })
    }

    pub async fn main_loop(&mut self) -> MainResult<()> {
        loop {
            tokio::select! {
                swarm_event = self.swarm.select_next_some() => {
                    tracing::warn!("swarm event: {swarm_event:#?}");
                    if let Some(event) = T::handle_swarm_event(self, swarm_event).await? {
                        tracing::warn!("passed off event to default handler");
                        self.handle_swarm_event(event).await?
                    }
                }
                Some(req) = self.rpc_thread.recv.recv() => {
                    let response = self.handle_rpc_request(req).await?;
                    self.rpc_thread.sender.send(response).await?;
                },
                Ok(event_opt) = self.inner.next_event() => {
                    if let Some(event) = event_opt {
                        tracing::warn!("inner event: {event:#?}");
                        T::handle_self_event(self, event).await?;
                    }
                },
            }
        }
    }

    /// This is where default swarm event handling should be implemented
    async fn handle_swarm_event(
        &mut self,
        event: impl Into<SwarmEvent<NodeBehaviourEvent>>,
    ) -> MainResult<()> {
        match Into::<SwarmEvent<NodeBehaviourEvent>>::into(event) {
            SwarmEvent::ConnectionEstablished { .. } => {}
            SwarmEvent::Behaviour(NodeBehaviourEvent::Gossip(
                libp2p::gossipsub::Event::Subscribed { peer_id, topic },
            )) if peer_id != *self.swarm.local_peer_id()
                && topic == NetworkTopic::ChainUpdate.publish() =>
            {
                self.swarm.behaviour_mut().as_mut().gossip.publish(
                    topic,
                    serde_json::to_vec(&self.blockchain).expect("failed to serialized blockchain"),
                )?;
            }
            SwarmEvent::Behaviour(NodeBehaviourEvent::Gossip(
                libp2p::gossipsub::Event::Message {
                    message: gossipsub::Message { topic, data, .. },
                    ..
                },
            )) if topic == NetworkTopic::ChainUpdate.publish() => {
                let chain: Blockchain = serde_json::from_slice(&data)?;

                if self.replace_chain(chain) {
                    tracing::warn!("replaced chain");
                } else {
                    tracing::warn!("did not replace chain");
                }

                // T::Behaviour::shared(self.swarm.behaviour_mut())
                //     .gossip
                //     .publish(topic, serde_json::to_vec(&self.blockchain)?)?;
            }

            _ => {}
        }
        Ok(())
    }

    /// Put chain validation logic here
    fn replace_chain(&mut self, potential_new_chain: Blockchain) -> bool {
        if self.blockchain.len() < potential_new_chain.len() {
            self.blockchain = potential_new_chain;
            return true;
        }
        false
    }

    fn swarm(keys: Keypair) -> MainResult<Swarm<T::Behaviour>> {
        let mut swarm = libp2p::SwarmBuilder::with_existing_identity(keys)
            .with_tokio()
            .with_quic()
            .with_dns()?
            .with_behaviour(|key| T::Behaviour::new(key.clone()))?
            .with_swarm_config(|cfg| {
                cfg.with_idle_connection_timeout(Duration::from_secs(u64::MAX))
            })
            .build();
        swarm
            .behaviour_mut()
            .as_mut()
            .gossip
            .subscribe(&NetworkTopic::ChainUpdate.subscribe())?;
        Ok(swarm)
    }
}

pub trait NodeTypeEvent: std::fmt::Debug {}

#[allow(async_fn_in_trait, private_bounds)]
pub trait NodeType: Debug {
    type Behaviour: NodeNetworkBehaviour;
    type Event: NodeTypeEvent;
    type RpcRequest: RpcRequestWrapper;
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
        _node: &mut Node<Self>,
        _e: SwarmEvent<<Self::Behaviour as NetworkBehaviour>::ToSwarm>,
    ) -> MainResult<Option<SwarmEvent<<Self::Behaviour as NetworkBehaviour>::ToSwarm>>>
    where
        Self: Sized,
    {
        Ok(Some(_e))
    }

    async fn handle_rpc_request(
        _node: &mut Node<Self>,
        _r: socket::Request,
    ) -> MainResult<OneOf<socket::Request, ProcessRequestResult>>
    where
        Self: Sized,
    {
        Ok(OneOf::Left(_r))
    }
}
