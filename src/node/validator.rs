use super::{NodeType, Wallet};
use crate::{
    behavior::{gossip::SysTopic, SysBehaviour, SysBehaviourEvent},
    chain::transaction::CompletedTransaction,
    MainResult,
};
use libp2p::{
    gossipsub::{self, Message},
    swarm::SwarmEvent,
    Swarm,
};
use tracing::warn;

#[derive(Debug)]
pub struct ValidatorNode {
    wallet: Wallet,
    tx_pool: Vec<CompletedTransaction>,
}

impl<'w> NodeType<'w> for ValidatorNode {
    // fn wallet_val(&'w mut self) -> &'w mut Wallet {
    //     &mut self.wallet
    // }
    fn init(swarm: &mut Swarm<SysBehaviour>) -> MainResult<Self> {
        warn!("creating validator node");
        swarm
            .behaviour_mut()
            .gossip
            .subscribe(&SysTopic::Completed.subscribe())?;
        Ok(ValidatorNode {
            wallet: Wallet::new(),
            tx_pool: vec![],
        })
    }

    async fn handle_swarm_event(
        node: &mut super::Node<Self>,
        event: SwarmEvent<SysBehaviourEvent>,
    ) -> MainResult<()> {
        match event {
            SwarmEvent::Behaviour(SysBehaviourEvent::Gossip(gossipsub::Event::Message {
                message: Message { data, topic, .. },
                ..
            })) if topic == SysTopic::Completed.publish() => {
                warn!("validator received transaction");
                let tx: CompletedTransaction = serde_json::from_slice(&data)?;
                node.typ.tx_pool.push(tx);
            }
            _ => {
                // warn!("unhandled validator event: {event:#?}");
            }
        }
        Ok(())
    }
}
