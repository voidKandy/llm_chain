use super::*;
use crate::behaviour::CompReqRes;
use crate::MainResult;
use behaviour::ClientNodeBehaviour;
use libp2p::swarm::NetworkBehaviour;
use libp2p::{gossipsub, identify, rendezvous};

use tokio::io::{AsyncBufReadExt, BufReader, Lines, Stdin};
pub struct ClientNode {
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
    type Behaviour = ClientNodeBehaviour;
    type Event = ClientNodeEvent;

    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            stdin: tokio::io::BufReader::new(tokio::io::stdin()).lines(),
        }
    }

    async fn next_event(&mut self) -> MainResult<Option<Self::Event>> {
        tokio::select! {
            // swarm_event = self.swarm.select_next_some() => Ok(Some(NodeEvent::from(swarm_event))),
            Ok(Some(line)) = self.stdin.next_line() => Ok(Some(ClientNodeEvent::UserInput(line).into())),

        }
    }

    async fn handle_self_event(&mut self, e: Self::Event) -> MainResult<()> {
        tracing::warn!("client event: {e:#?}");
        Ok(())
    }
}
