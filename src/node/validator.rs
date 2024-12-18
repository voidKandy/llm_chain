use super::*;
use crate::{blockchain::transaction::transfer::Transfer, util::map_vec::MapVec, MainResult};
use behaviour::ServerNodeBehaviour;

/// Does some basic POW and validates blocks
pub struct ValidatorNode {
    mempool: MapVec<String, Transfer>,
}

#[derive(Debug)]
pub enum ValidatorNodeEvent {}
impl NodeTypeEvent for ValidatorNodeEvent {}

impl NodeType for ValidatorNode {
    type Behaviour = ServerNodeBehaviour;
    type Event = ValidatorNodeEvent;

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
