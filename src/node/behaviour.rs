use libp2p::{
    gossipsub::{self, MessageAuthenticity},
    identify,
    identity::Keypair,
    rendezvous,
    request_response::{self, ProtocolSupport},
    swarm::NetworkBehaviour,
    StreamProtocol,
};
use std::hash::{DefaultHasher, Hash, Hasher};

use crate::util::behaviour::{
    req_res::{NetworkReqRes, NetworkRequest, NetworkResponse},
    IDENTIFY_ID,
};

/// Behaviour that is shared between server/client
/// Should never be manually instantiated
#[derive(NetworkBehaviour)]
pub struct SharedBehaviour {
    pub gossip: gossipsub::Behaviour,
    pub identify: identify::Behaviour,
    pub req_res: NetworkReqRes,
    pub stream: libp2p_stream::Behaviour,
}

#[derive(Debug)]
pub enum NodeBehaviourEvent {
    Identify(identify::Event),
    Gossip(gossipsub::Event),
    ReqRes(request_response::Event<NetworkRequest, NetworkResponse>),
    RendezvousServer(rendezvous::server::Event),
    RendezvousClient(rendezvous::client::Event),
    /// Stream event emits ()
    Stream(()),
}

impl From<SharedBehaviourEvent> for NodeBehaviourEvent {
    fn from(value: SharedBehaviourEvent) -> Self {
        match value {
            SharedBehaviourEvent::Gossip(e) => Self::from(e),
            SharedBehaviourEvent::Identify(e) => Self::from(e),
            SharedBehaviourEvent::ReqRes(e) => Self::from(e),
            SharedBehaviourEvent::Stream(e) => Self::Stream(e),
        }
    }
}

impl From<identify::Event> for NodeBehaviourEvent {
    fn from(value: identify::Event) -> Self {
        Self::Identify(value)
    }
}
impl From<request_response::Event<NetworkRequest, NetworkResponse>> for NodeBehaviourEvent {
    fn from(value: request_response::Event<NetworkRequest, NetworkResponse>) -> Self {
        Self::ReqRes(value)
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

impl SharedBehaviour {
    pub fn new(keys: Keypair) -> Self {
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

        let req_res =
            libp2p::request_response::json::Behaviour::<NetworkRequest, NetworkResponse>::new(
                [(
                    StreamProtocol::new("/networkreqres/1.0.0"),
                    ProtocolSupport::Full,
                )],
                libp2p::request_response::Config::default(),
            );

        let stream = libp2p_stream::Behaviour::new();
        Self {
            gossip,
            identify,
            req_res,
            stream,
        }
    }
}

/// Any behaviour that a node can possibly have must implement this
pub trait NodeNetworkBehaviour:
    AsRef<SharedBehaviour> + AsMut<SharedBehaviour> + NetworkBehaviour
{
    fn new(keys: Keypair) -> Self
    where
        Self: Sized;
}
