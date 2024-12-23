use core::node::behaviour::{NodeBehaviourEvent, NodeNetworkBehaviour, SharedBehaviour};
use libp2p::{identity::Keypair, swarm::NetworkBehaviour};

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "NodeBehaviourEvent")]
pub struct ServerNodeBehaviour {
    pub shared: SharedBehaviour,
    pub rendezvous: libp2p::rendezvous::server::Behaviour,
}

impl AsRef<SharedBehaviour> for ServerNodeBehaviour {
    fn as_ref(&self) -> &SharedBehaviour {
        &self.shared
    }
}

impl AsMut<SharedBehaviour> for ServerNodeBehaviour {
    fn as_mut(&mut self) -> &mut SharedBehaviour {
        &mut self.shared
    }
}

impl NodeNetworkBehaviour for ServerNodeBehaviour {
    fn new(keys: Keypair) -> Self
    where
        Self: Sized,
    {
        let shared = SharedBehaviour::new(keys);
        Self {
            shared,
            rendezvous: libp2p::rendezvous::server::Behaviour::new(
                libp2p::rendezvous::server::Config::default(),
            ),
        }
    }
}
