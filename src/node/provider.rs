use super::NodeType;
use crate::{behavior::SysBehaviour, MainResult, MODEL_ID_0};
use libp2p::{
    kad::{Mode, RecordKey},
    Swarm,
};
use tracing::warn;

#[derive(Debug)]
pub struct ProviderNode;

impl NodeType for ProviderNode {
    fn init(swarm: &mut Swarm<SysBehaviour>) -> MainResult<Self> {
        // this could be causing bug!
        swarm.behaviour_mut().kad.set_mode(Some(Mode::Server));
        // completely arbitrary rn, not quite sure how to implement model providing yet
        let key = RecordKey::new(&MODEL_ID_0);

        let _ = swarm
            .behaviour_mut()
            .kad
            .start_providing(key.clone())
            .expect("failed to make node a provider");

        Ok(ProviderNode)
    }
}
