use core::blockchain::transaction::transfer::Transfer;
use core::util::map_vec::*;
use core::{node::*, MainResult};
use libp2p::Swarm;
use rpc::RequestWrapper;

use crate::behaviour::ServerNodeBehaviour;

/// Does some basic POW and validates blocks
#[derive(Debug)]
pub struct MinerNode {
    mempool: MapVec<String, Transfer>,
}

#[derive(Debug)]
pub enum MinerNodeEvent {}
impl NodeTypeEvent for MinerNodeEvent {}

impl NodeType for MinerNode {
    type Behaviour = ServerNodeBehaviour;
    type Event = MinerNodeEvent;
    type RpcRequest = RequestWrapper;

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
