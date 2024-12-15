use super::*;
use crate::behaviour::{CompConnect, CompConnectConfirm, CompReqRes, IDENTIFY_ID};
use crate::MainResult;
use futures::StreamExt;
use libp2p::gossipsub::MessageAuthenticity;
use libp2p::identity::Keypair;
use libp2p::request_response::ProtocolSupport;
use libp2p::swarm::NetworkBehaviour;
use libp2p::{gossipsub, identify, rendezvous, request_response, StreamProtocol, Swarm};
use std::hash::{DefaultHasher, Hash, Hasher};

use tokio::io::{AsyncBufReadExt, BufReader, Lines, Stdin};
pub struct ClientNode {
    swarm: Swarm<ClientNodeBehavior>,
    stdin: Lines<BufReader<Stdin>>,
}

#[derive(NetworkBehaviour)]
pub struct ClientNodeBehavior {
    pub gossip: gossipsub::Behaviour,
    pub rendezvous: rendezvous::client::Behaviour,
    pub identify: identify::Behaviour,
    pub req_res: CompReqRes,
}

#[derive(Debug)]
pub enum ClientNodeEvent {
    UserInput(String),
}

impl NodeTypeEvent for ClientNodeEvent {}
impl NodeType for ClientNode {
    type Behaviour = ClientNodeBehavior;
    type Event = ClientNodeEvent;
    fn swarm_mut(&mut self) -> &mut Swarm<ClientNodeBehavior> {
        &mut self.swarm
    }

    fn from_swarm(swarm: Swarm<ClientNodeBehavior>) -> Self
    where
        Self: Sized,
    {
        Self {
            swarm,
            stdin: tokio::io::BufReader::new(tokio::io::stdin()).lines(),
        }
    }

    async fn next_event(&mut self) -> MainResult<Option<NodeEvent<Self::Behaviour, Self::Event>>> {
        tokio::select! {
            swarm_event = self.swarm.select_next_some() => Ok(Some(NodeEvent::from(swarm_event))),
            Ok(Some(line)) = self.stdin.next_line() => Ok(Some(ClientNodeEvent::UserInput(line).into())),

        }
    }

    async fn handle_event(&mut self, e: NodeEvent<Self::Behaviour, Self::Event>) -> MainResult<()> {
        tracing::warn!("client event: {e:#?}");
        Ok(())
    }

    fn behaviour(keys: &Keypair) -> ClientNodeBehavior {
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

        ClientNodeBehavior {
            gossip,
            rendezvous: rendezvous::client::Behaviour::new(keys.clone()),
            identify,
            req_res,
        }
    }
}
