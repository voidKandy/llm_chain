mod behavior;
mod chain;
mod node;
mod telemetry;
use behavior::SysBehaviour;
use chain::block::GENESIS_BLOCK;
use clap::Parser;
use futures::StreamExt;
use libp2p::{
    gossipsub,
    identity::Keypair,
    kad::{Mode, RecordKey},
    Multiaddr, PeerId,
};
use node::{Node, NodeType};
use std::{sync::LazyLock, thread::sleep, time::Duration};
use telemetry::TRACING;
use tracing::warn;

type MainResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(name = "Dial Address")]
    pub dial_addr: Option<String>,
    /// if true, provider
    /// if false, validator
    /// if none, client
    #[arg(short = 's')]
    pub server: Option<bool>,
}

// these should be mapped to real models down the line
const MODEL_ID_0: &str = "model_0";
const MODEL_ID_1: &str = "model_1";

static KEYS: LazyLock<Keypair> = LazyLock::new(|| Keypair::generate_ed25519());
static PEER_ID: LazyLock<PeerId> = LazyLock::new(|| PeerId::from(KEYS.public()));
pub const CHAIN_TOPIC: &str = "chain_updates";
pub const TX_TOPIC: &str = "transactions";

#[tokio::main]
async fn main() -> MainResult<()> {
    LazyLock::force(&TRACING);
    let args = Args::parse();
    warn!("args: {args:#?}");
    let local_id = LazyLock::force(&PEER_ID);

    warn!("Peer Id: {}", &local_id);
    let mut swarm = libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_quic()
        // .with_tcp(
        //     libp2p::tcp::Config::default(),
        //     libp2p::tls::Config::new,
        //     libp2p::yamux::Config::default,
        // )?
        .with_behaviour(|key| SysBehaviour::new(*local_id, key.clone()))?
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(u64::MAX)))
        .build();

    let chain_topic = gossipsub::IdentTopic::new(CHAIN_TOPIC);
    swarm.behaviour_mut().gossip.subscribe(&chain_topic)?;
    let mut node = {
        match args.server {
            Some(true) => {
                swarm.behaviour_mut().kad.set_mode(Some(Mode::Server));
                // completely arbitrary rn, not quite sure how to implement model
                // providing yet
                let key = RecordKey::new(&MODEL_ID_0);

                let qid = swarm
                    .behaviour_mut()
                    .kad
                    .start_providing(key.clone())
                    .expect("failed to make node a provider");

                warn!("making this node a server, qid: {qid:#?}");

                // temporary behavior to make testing easier
                let b = &GENESIS_BLOCK;
                let gen_block = LazyLock::force(b).clone();
                Node::new(NodeType::Provider, vec![gen_block])
            }

            Some(false) => {
                let tx_topic = gossipsub::IdentTopic::new(TX_TOPIC);
                swarm.behaviour_mut().gossip.subscribe(&tx_topic)?;
                warn!("creating validator node");
                Node::new(NodeType::Validator { tx_pool: vec![] }, vec![])
            }

            None => {
                warn!("creating client node");
                Node::new(NodeType::new_client(), vec![])
            }
        }
    };

    swarm.listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse()?)?;
    if let Some(addr) = args.dial_addr {
        let remote: Multiaddr = addr.parse()?;
        swarm.dial(remote)?;
        warn!("Dialed {addr}")
    }

    loop {
        let _ = swarm
            .behaviour_mut()
            .gossip
            .publish(chain_topic.clone(), node.ledger_bytes()?);
        match node.typ {
            NodeType::Provider | NodeType::Validator { .. } => {
                tokio::select! {
                    event = swarm.select_next_some() => node.handle_swarm_event(&mut swarm, event).await
                }
            }
            NodeType::Client {
                ref mut stdin,
                ref mut provider_query_id,
                ref mut user_input,
            } => {
                let key = RecordKey::new(&MODEL_ID_0);
                if provider_query_id.is_none() {
                    *provider_query_id = Some(swarm.behaviour_mut().kad.get_providers(key.clone()));
                }
                tokio::select! {
                    // Ok(Some(line)) = stdin.next_line() => {
                        // if provider_query_id.is_none() {
                        //     *user_input = Some(line);
                        //
                        //      *provider_query_id = Some(swarm
                        //         .behaviour_mut()
                        //         .kad
                        //         .get_providers(key.clone()));
                        // }

                    // }
                    event = swarm.select_next_some() => node.handle_swarm_event(&mut swarm, event).await
                }
            }
        }
    }
}
