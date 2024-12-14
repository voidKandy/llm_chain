use super::*;
use crate::behavior::gossip::{self, CompConnect, CompConnectConfirm};
use crate::behavior::IDENTIFY_ID;
use crate::MainResult;
use futures::StreamExt;
use libp2p::gossipsub::MessageAuthenticity;
use libp2p::identity::Keypair;
use libp2p::request_response::ProtocolSupport;
use libp2p::swarm::NetworkBehaviour;
use libp2p::{gossipsub, identify, rendezvous, request_response, StreamProtocol, Swarm};
use std::hash::{DefaultHasher, Hash, Hasher};

pub struct ServerNode {
    swarm: Swarm<ServerNodeBehavior>,
}

#[derive(NetworkBehaviour)]
pub struct ServerNodeBehavior {
    pub gossip: gossipsub::Behaviour,
    pub rendezvous: rendezvous::server::Behaviour,
    pub identify: identify::Behaviour,
    pub req_res: gossip::CompReqRes,
}

#[derive(Debug)]
pub enum ServerNodeEvent {}
impl NodeTypeEvent for ServerNodeEvent {}

impl NodeType for ServerNode {
    type Behaviour = ServerNodeBehavior;
    type Event = ServerNodeEvent;

    fn from_swarm(swarm: Swarm<ServerNodeBehavior>) -> Self
    where
        Self: Sized,
    {
        Self { swarm }
    }

    fn swarm_mut(&mut self) -> &mut Swarm<ServerNodeBehavior> {
        &mut self.swarm
    }

    async fn next_event(&mut self) -> MainResult<Option<NodeEvent<Self::Behaviour, Self::Event>>> {
        tokio::select! {
            swarm_event = self.swarm.select_next_some() => {
                Ok(Some(NodeEvent::from(swarm_event)))
            }
        }
    }

    fn behaviour(keys: &Keypair) -> ServerNodeBehavior {
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
            gossipsub::Behaviour::new(MessageAuthenticity::Signed(keys.clone()), gossip_config)
                .unwrap();

        let req_res = request_response::json::Behaviour::<CompConnect, CompConnectConfirm>::new(
            [(
                StreamProtocol::new("/compreqres/1.0.0"),
                ProtocolSupport::Full,
            )],
            request_response::Config::default(),
        );

        ServerNodeBehavior {
            gossip,
            rendezvous: rendezvous::server::Behaviour::new(rendezvous::server::Config::default()),
            identify,
            req_res,
        }
    }
}
