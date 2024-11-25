pub mod client;
pub mod provider;
pub mod validator;
use crate::{
    behavior::{SysBehaviour, SysBehaviourEvent},
    chain::block::{Block, Blockchain},
    MainResult, CHAIN_TOPIC,
};
use futures::StreamExt;
use libp2p::{
    gossipsub::{self, Message, TopicHash},
    identity::Keypair,
    swarm::SwarmEvent,
    Multiaddr, PeerId, Swarm,
};
use std::{fmt::Debug, time::Duration};
use tracing::warn;

pub struct Node<T> {
    swarm: Swarm<SysBehaviour>,
    typ: T,
    pub ledger: Vec<Block>,
}

pub(super) trait NodeType: Sized + Debug {
    fn init(swarm: &mut Swarm<SysBehaviour>) -> MainResult<Self>;
    async fn tokio_select_branches(node: &mut Node<Self>) -> MainResult<()> {
        tokio::select! {
            event = node.swarm.select_next_some() => {
                Self::default_handle_swarm_event(node, event).await
            }
        }
    }
    async fn handle_swarm_event(
        _node: &mut Node<Self>,
        event: SwarmEvent<SysBehaviourEvent>,
    ) -> MainResult<()> {
        warn!("unhandled event: {event:#?}");
        Ok(())
    }

    // Event matching in this has to be VERY explicit, otherwise branches for events handled by particular node
    // types wont be reached
    async fn default_handle_swarm_event(
        node: &mut Node<Self>,
        event: SwarmEvent<SysBehaviourEvent>,
    ) -> MainResult<()> {
        match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                warn!("Listening on {address:?}");
                // node.swarm
                //     .add_peer_address(*node.swarm.local_peer_id(), address);
                // node.swarm
                //     .behaviour_mut()
                //     .kad
                //     .add_address(node.swarm.local_peer_id(), address);
                // node.swarm
                //     .behaviour_mut()
                //     .req_res
                //     .add_address(*node.swarm.local_peer_id(), address);
            }

            SwarmEvent::Behaviour(SysBehaviourEvent::Gossip(gossipsub::Event::Message {
                message: Message { data, topic, .. },
                ..
            })) if topic == TopicHash::from_raw(CHAIN_TOPIC) => {
                let update: Vec<Block> =
                    serde_json::from_slice(&data).expect("failed to deserialize chain update");
                warn!("received chain update: {update:#?}");
                if node.replace_ledger(update) {
                    warn!("replaced node's ledger");
                } else {
                    warn!("did not replace node's ledger");
                }
            }
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                warn!("New connection to peer: {peer_id:#?}")
            }
            SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                warn!("Closed connection to peer: {peer_id:#?}\ncause: {cause:#?}")
            }
            e => return Self::handle_swarm_event(node, e).await,
        }
        Ok(())
    }
}

impl<T> Node<T>
where
    T: NodeType,
{
    /// Create a node and start listening
    pub fn init(dial_address: Option<Multiaddr>, blockchain: Blockchain) -> MainResult<Node<T>> {
        let keys = Keypair::generate_ed25519();
        let local_id = PeerId::from(keys.public());
        let mut swarm = libp2p::SwarmBuilder::with_existing_identity(keys)
            .with_tokio()
            .with_quic()
            // .with_tcp(
            //     libp2p::tcp::Config::default(),
            //     libp2p::tls::Config::new,
            //     libp2p::yamux::Config::default,
            // )?
            .with_dns()?
            .with_behaviour(|key| SysBehaviour::new(key.clone()))?
            .with_swarm_config(|cfg| {
                cfg.with_idle_connection_timeout(Duration::from_secs(u64::MAX))
            })
            .build();

        warn!("LOCAL ID: {}", &local_id);

        let chain_topic = gossipsub::IdentTopic::new(CHAIN_TOPIC);
        swarm.behaviour_mut().gossip.subscribe(&chain_topic)?;
        let typ = T::init(&mut swarm)?;

        swarm.listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse()?)?;
        // swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

        if let Some(addr) = dial_address {
            warn!("Dialing {addr}");
            swarm.dial(addr)?;
        }

        Ok(Node {
            swarm,
            typ,
            ledger: blockchain,
        })
    }

    pub async fn main_loop(&mut self) -> MainResult<()> {
        loop {
            // let _ = self
            //     .swarm
            //     .behaviour_mut()
            //     .gossip
            //     .publish(chain_topic.clone(), self.ledger_bytes()?);
            T::tokio_select_branches(self).await?
        }
    }

    fn ledger_bytes(&self) -> serde_json::Result<Vec<u8>> {
        serde_json::to_vec(&self.ledger)
    }

    /// If other ledger is longer, replace mine with other.
    /// Returns whether or not ledger was replaced
    pub fn replace_ledger(&mut self, other: Vec<Block>) -> bool {
        let replace = other.len() > self.ledger.len();
        if replace {
            self.ledger = other;
        }

        replace
    }
}
