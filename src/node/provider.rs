use super::*;
use crate::MainResult;
use behaviour::ServerNodeBehaviour;

/// Does some basic POW and validates blocks
pub struct ProviderNode {}

#[derive(Debug)]
pub enum ProviderNodeEvent {}
impl NodeTypeEvent for ProviderNodeEvent {}

impl NodeType for ProviderNode {
    type Behaviour = ServerNodeBehaviour;
    type Event = ProviderNodeEvent;
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {}
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

    async fn handle_self_event(&mut self, e: Self::Event) -> MainResult<()> {
        tracing::warn!("server event: {e:#?}");
        Ok(())
    }
}
