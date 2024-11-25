use super::NodeType;
use crate::{
    behavior::{SysBehaviour, SysBehaviourEvent},
    chain::transaction::Transaction,
    MainResult, TX_TOPIC,
};
use libp2p::{
    gossipsub::{self, Message, TopicHash},
    swarm::SwarmEvent,
    Swarm,
};
use tracing::warn;

#[derive(Debug)]
pub struct ValidatorNode {
    tx_pool: Vec<Transaction>,
}

impl NodeType for ValidatorNode {
    fn init(swarm: &mut Swarm<SysBehaviour>) -> MainResult<Self> {
        let tx_topic = gossipsub::IdentTopic::new(TX_TOPIC);
        swarm.behaviour_mut().gossip.subscribe(&tx_topic)?;
        warn!("creating validator node");
        Ok(ValidatorNode { tx_pool: vec![] })
    }

    async fn handle_swarm_event(
        node: &mut super::Node<Self>,
        event: SwarmEvent<SysBehaviourEvent>,
    ) -> MainResult<()> {
        match event {
            SwarmEvent::Behaviour(SysBehaviourEvent::Gossip(gossipsub::Event::Message {
                message: Message { data, topic, .. },
                ..
            })) if topic == TopicHash::from_raw(TX_TOPIC) => {
                warn!("validator received transaction");
                let tx: Transaction = serde_json::from_slice(&data)?;
                node.typ.tx_pool.push(tx);
            }
            _ => {
                warn!("unhandled validator event: {event:#?}");
            }
        }
        Ok(())
    }
}
