use std::hash::{DefaultHasher, Hash, Hasher};

use crate::{behaviour::IDENTIFY_ID, MainResult};
use libp2p::{
    gossipsub::{self, MessageAuthenticity},
    identify,
    identity::Keypair,
    rendezvous,
    swarm::NetworkBehaviour,
};

/// Behaviour that is shared between server/client
/// Should never be manually instantiated
#[derive(NetworkBehaviour)]
pub struct SharedBehaviour {
    pub gossip: gossipsub::Behaviour,
    pub identify: identify::Behaviour,
}

impl SharedBehaviour {
    fn new(keys: Keypair) -> Self {
        let message_id_fn = |message: &gossipsub::Message| {
            let mut s = DefaultHasher::new();
            message.data.hash(&mut s);
            gossipsub::MessageId::from(s.finish().to_string())
        };
        let identify = identify::Behaviour::new(identify::Config::new(
            IDENTIFY_ID.to_string(),
            keys.public(),
        ));

        let gossip_config = gossipsub::ConfigBuilder::default()
            .message_id_fn(message_id_fn)
            .build()
            .expect("failed to build gossip config");

        let gossip =
            gossipsub::Behaviour::new(MessageAuthenticity::Signed(keys), gossip_config).unwrap();

        // let req_res = request_response::json::Behaviour::<CompConnect, CompConnectConfirm>::new(
        //     [(
        //         StreamProtocol::new("/compreqres/1.0.0"),
        //         ProtocolSupport::Full,
        //     )],
        //     request_response::Config::default(),
        // );
        Self { gossip, identify }
    }
}

/// Any behaviour that a node can possibly have must implement this
pub trait NodeNetworkBehaviour: NetworkBehaviour {
    fn new(keys: Keypair) -> Self
    where
        Self: Sized;
}

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "NodeBehaviourEvent")]
pub struct ServerNodeBehaviour {
    pub shared: SharedBehaviour,
    pub rendezvous: rendezvous::server::Behaviour,
}
impl NodeNetworkBehaviour for ServerNodeBehaviour {
    fn new(keys: Keypair) -> Self
    where
        Self: Sized,
    {
        let shared = SharedBehaviour::new(keys);
        Self {
            shared,
            rendezvous: rendezvous::server::Behaviour::new(rendezvous::server::Config::default()),
        }
    }
}

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "NodeBehaviourEvent")]
pub struct ClientNodeBehaviour {
    pub shared: SharedBehaviour,
    pub rendezvous: rendezvous::client::Behaviour,
}
impl NodeNetworkBehaviour for ClientNodeBehaviour {
    fn new(keys: Keypair) -> Self
    where
        Self: Sized,
    {
        let shared = SharedBehaviour::new(keys.clone());
        Self {
            shared,
            rendezvous: rendezvous::client::Behaviour::new(keys.clone()),
        }
    }
}

#[derive(Debug)]
pub enum NodeBehaviourEvent {
    Identify(identify::Event),
    Gossip(gossipsub::Event),
    RendezvousServer(rendezvous::server::Event),
    RendezvousClient(rendezvous::client::Event),
}

impl From<SharedBehaviourEvent> for NodeBehaviourEvent {
    fn from(value: SharedBehaviourEvent) -> Self {
        match value {
            SharedBehaviourEvent::Gossip(e) => Self::from(e),
            SharedBehaviourEvent::Identify(e) => Self::from(e),
        }
    }
}
impl From<identify::Event> for NodeBehaviourEvent {
    fn from(value: identify::Event) -> Self {
        Self::Identify(value)
    }
}
impl From<gossipsub::Event> for NodeBehaviourEvent {
    fn from(value: gossipsub::Event) -> Self {
        Self::Gossip(value)
    }
}
impl From<rendezvous::client::Event> for NodeBehaviourEvent {
    fn from(value: rendezvous::client::Event) -> Self {
        Self::RendezvousClient(value)
    }
}
impl From<rendezvous::server::Event> for NodeBehaviourEvent {
    fn from(value: rendezvous::server::Event) -> Self {
        Self::RendezvousServer(value)
    }
}
