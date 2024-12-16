pub mod behaviour;
pub mod client;
pub mod provider;
pub mod validator;
use crate::MainResult;
use behaviour::{NodeBehaviourEvent, NodeNetworkBehaviour};
use futures::StreamExt;
use libp2p::{
    identity::Keypair,
    swarm::{NetworkBehaviour, Swarm, SwarmEvent},
};
use std::time::Duration;

pub struct Node<T: NodeType> {
    keys: Keypair,
    pub swarm: Swarm<T::Behaviour>,
    pub inner: T,
}

impl<T> Node<T>
where
    T: NodeType,
    <<T as NodeType>::Behaviour as NetworkBehaviour>::ToSwarm: std::fmt::Debug,
{
    pub fn try_from_keys(keys: Keypair) -> MainResult<Self> {
        let swarm = Self::swarm(keys.clone())?;
        let inner = T::new();
        Ok(Self { inner, swarm, keys })
    }

    pub async fn main_loop(&mut self) -> MainResult<()> {
        loop {
            tokio::select! {
                swarm_event = self.swarm.select_next_some() => {
                    tracing::warn!("swarm event: {swarm_event:#?}");
                }
                Ok(Some(inner_event)) = self.inner.next_event() => {
                    tracing::warn!("inner event: {inner_event:#?}");
                }
            }
        }
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
    fn new() -> Self
    where
        Self: Sized;

    async fn next_event(&mut self) -> MainResult<Option<Self::Event>>;
    async fn handle_self_event(&mut self, e: Self::Event) -> MainResult<()>;
    async fn handle_swarm_event(&mut self, _e: SwarmEvent<NodeBehaviourEvent>) -> MainResult<()> {
        Ok(())
    }
}
