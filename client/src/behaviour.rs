use core::node::behaviour::{NodeBehaviourEvent, NodeNetworkBehaviour, SharedBehaviour};
use libp2p::swarm::NetworkBehaviour;

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "NodeBehaviourEvent")]
pub struct ClientNodeBehaviour {
    pub shared: SharedBehaviour,
    pub rendezvous: libp2p::rendezvous::client::Behaviour,
}

impl AsRef<SharedBehaviour> for ClientNodeBehaviour {
    fn as_ref(&self) -> &SharedBehaviour {
        &self.shared
    }
}

impl AsMut<SharedBehaviour> for ClientNodeBehaviour {
    fn as_mut(&mut self) -> &mut SharedBehaviour {
        &mut self.shared
    }
}

impl NodeNetworkBehaviour for ClientNodeBehaviour {
    fn new(keys: libp2p::identity::Keypair) -> Self
    where
        Self: Sized,
    {
        let shared = SharedBehaviour::new(keys.clone());
        Self {
            shared,
            rendezvous: libp2p::rendezvous::client::Behaviour::new(keys),
        }
    }
}
