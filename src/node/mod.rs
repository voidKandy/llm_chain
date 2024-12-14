pub mod client;
pub mod server;
use crate::MainResult;
use libp2p::{
    identity::Keypair,
    swarm::{NetworkBehaviour, Swarm, SwarmEvent},
};
use std::time::Duration;

pub struct Node<T> {
    keys: Keypair,
    pub inner: T,
}

impl<T> Node<T>
where
    T: NodeType,
{
    pub fn try_from_keys(keys: Keypair) -> MainResult<Self> {
        let swarm = T::swarm(keys.clone())?;
        let inner = T::from_swarm(swarm);
        Ok(Self { inner, keys })
    }
}

pub enum NodeEvent<B: NetworkBehaviour, E: NodeTypeEvent> {
    Swarm(SwarmEvent<B::ToSwarm>),
    NodeType(E),
}

impl<B, E> std::fmt::Debug for NodeEvent<B, E>
where
    B: NetworkBehaviour,
    E: NodeTypeEvent,
    B::ToSwarm: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // keep these separate to avoid recursion
        match self {
            Self::Swarm(e) => write!(f, "{e:?}"),
            Self::NodeType(e) => write!(f, "{e:?}"),
        }
    }
}

impl<B: NetworkBehaviour, E: NodeTypeEvent> From<SwarmEvent<B::ToSwarm>> for NodeEvent<B, E> {
    fn from(value: SwarmEvent<B::ToSwarm>) -> Self {
        Self::Swarm(value)
    }
}

impl<B: NetworkBehaviour, E: NodeTypeEvent> From<E> for NodeEvent<B, E> {
    fn from(value: E) -> Self {
        Self::NodeType(value)
    }
}

pub trait NodeTypeEvent: std::fmt::Debug {}

#[allow(async_fn_in_trait, private_bounds)]
pub trait NodeType {
    type Behaviour: NetworkBehaviour;
    type Event: NodeTypeEvent;
    fn behaviour(keys: &Keypair) -> Self::Behaviour;

    fn swarm_mut(&mut self) -> &mut Swarm<Self::Behaviour>;
    fn swarm(keys: Keypair) -> MainResult<Swarm<Self::Behaviour>> {
        Ok(libp2p::SwarmBuilder::with_existing_identity(keys.clone())
            .with_tokio()
            .with_quic()
            .with_dns()?
            .with_behaviour(|key| Self::behaviour(key))?
            .with_swarm_config(|cfg| {
                cfg.with_idle_connection_timeout(Duration::from_secs(u64::MAX))
            })
            .build())
    }

    async fn next_event(&mut self) -> MainResult<Option<NodeEvent<Self::Behaviour, Self::Event>>>;

    fn from_swarm(swarm: Swarm<Self::Behaviour>) -> Self
    where
        Self: Sized;
}
