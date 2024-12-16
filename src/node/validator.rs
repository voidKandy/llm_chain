use super::*;
use crate::MainResult;
use behaviour::ServerNodeBehaviour;

/// Does some basic POW and validates blocks
pub struct ValidatorNode {}

#[derive(Debug)]
pub enum ValidatorNodeEvent {}
impl NodeTypeEvent for ValidatorNodeEvent {}

impl NodeType for ValidatorNode {
    type Behaviour = ServerNodeBehaviour;
    type Event = ValidatorNodeEvent;
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
