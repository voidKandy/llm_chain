use libp2p::Swarm;
use llm_chain::blockchain::transaction::transfer::Transfer;
use llm_chain::util::map_vec::*;
use llm_chain::{node::*, MainResult};

use crate::behaviour::ServerNodeBehaviour;

/// provides model work
pub struct ProviderNode;

#[derive(Debug)]
pub enum ProviderNodeEvent {}
impl NodeTypeEvent for ProviderNodeEvent {}

/// Does some basic POW and validates blocks
pub struct MinerNode {
    mempool: MapVec<String, Transfer>,
}

#[derive(Debug)]
pub enum MinerNodeEvent {}
impl NodeTypeEvent for MinerNodeEvent {}

impl NodeType for ProviderNode {
    type Behaviour = ServerNodeBehaviour;
    type Event = ProviderNodeEvent;
    fn init_with_swarm(swarm: &mut Swarm<Self::Behaviour>) -> MainResult<Self>
    where
        Self: Sized,
    {
        Ok(Self)
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
}

impl NodeType for MinerNode {
    type Behaviour = ServerNodeBehaviour;
    type Event = MinerNodeEvent;

    fn init_with_swarm(swarm: &mut Swarm<Self::Behaviour>) -> MainResult<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            mempool: MapVec::new(),
        })
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
}
