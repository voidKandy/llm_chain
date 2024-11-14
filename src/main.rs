mod telemetry;
use clap::Parser;
use futures::StreamExt;
use libp2p::{
    floodsub::{Floodsub, Topic},
    gossipsub::{self, MessageAuthenticity},
    identity::Keypair,
    kad::{self, store::MemoryStore, Mode, RecordKey},
    swarm::{behaviour, NetworkBehaviour, SwarmEvent},
    Multiaddr, PeerId,
};
use serde::{Deserialize, Serialize};
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    sync::LazyLock,
    time::Duration,
};
use telemetry::TRACING;
use tokio::io::{AsyncBufReadExt, BufReader, Lines, Stdin};
use tracing::warn;

type MainResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(name = "Dial Address")]
    pub dial_addr: Option<String>,
    #[arg(short = 's')]
    pub server: bool,
}

static KEYS: LazyLock<Keypair> = LazyLock::new(|| Keypair::generate_ed25519());
static PEER_ID: LazyLock<PeerId> = LazyLock::new(|| PeerId::from(KEYS.public()));

#[derive(Debug, Deserialize, Serialize)]
enum Message<'m> {
    CompletionReq { model_id: u8, prompt: &'m str },
}

#[derive(NetworkBehaviour)]
struct Behaviour {
    gossip: gossipsub::Behaviour,
    kad: kad::Behaviour<MemoryStore>,
}

impl Behaviour {
    fn new(peer_id: PeerId, key: Keypair) -> Self {
        let message_id_fn = |message: &gossipsub::Message| {
            let mut s = DefaultHasher::new();
            message.data.hash(&mut s);
            gossipsub::MessageId::from(s.finish().to_string())
        };

        let peer_store = MemoryStore::new(peer_id);
        let gossip_config = gossipsub::ConfigBuilder::default()
            .message_id_fn(message_id_fn)
            .build()
            .expect("failed to build gossip config");
        let gossip =
            gossipsub::Behaviour::new(MessageAuthenticity::Signed(key), gossip_config).unwrap();
        Behaviour {
            gossip,
            kad: kad::Behaviour::<MemoryStore>::new(peer_id, peer_store),
        }
    }
}

async fn read_stdin_opt(stdinopt: &mut Option<Lines<BufReader<Stdin>>>) -> Option<String> {
    if let Some(stdin) = stdinopt.as_mut() {
        return stdin.next_line().await.expect("failed to read stdin");
    }
    None
}

#[tokio::main]
async fn main() -> MainResult<()> {
    LazyLock::force(&TRACING);
    let args = Args::parse();
    warn!("args: {args:#?}");
    let local_id = LazyLock::force(&PEER_ID);

    warn!("Peer Id: {}", &local_id);

    let mut swarm = libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default(),
            libp2p::tls::Config::new,
            libp2p::yamux::Config::default,
        )?
        .with_behaviour(|key| Behaviour::new(*local_id, key.clone()))?
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(u64::MAX)))
        .build();

    let topic = gossipsub::IdentTopic::new("test-net");
    swarm.behaviour_mut().gossip.subscribe(&topic)?;
    // THIS IS HOW YOU DO HETEROGENEOUS
    let mut stdinopt = None;
    if args.server {
        warn!("making this node a server");
        swarm.behaviour_mut().kad.set_mode(Some(Mode::Server));

        let id = swarm
            .behaviour_mut()
            .kad
            .start_providing(RecordKey::new(b"some_model"))
            .expect("failed to make node a provider");
    } else {
        stdinopt = Some(tokio::io::BufReader::new(tokio::io::stdin()).lines());
    }

    // Tell the swarm to listen on all interfaces and a random, OS-assigned
    // port.
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    // Dial the peer identified by the multi-address given as the second
    // command-line argument, if any.
    if let Some(addr) = args.dial_addr {
        let remote: Multiaddr = addr.parse()?;
        swarm.dial(remote)?;
        warn!("Dialed {addr}")
    }

    loop {
        tokio::select! {
                Some(line) = read_stdin_opt(&mut stdinopt) => {
                    let message = Message::CompletionReq{model_id: 0, prompt: &line};
                    let json_byte_vec = serde_json::to_vec(&message).expect("failed serialization of message");
                    if let Err(e) = swarm
                        .behaviour_mut().gossip
                        .publish(topic.clone(), json_byte_vec) {
                        warn!("Publish error: {e:?}");
                    }
                }

            event = swarm.select_next_some() => match event {

            SwarmEvent::NewListenAddr { address, .. } => warn!("Listening on {address:?}"),
            SwarmEvent::Behaviour(beh_event) => {
                match beh_event {
                    BehaviourEvent::Gossip(gossipsub::Event::Message {
                        propagation_source,
                        message_id,
                        message
                    }) => {
                        let deserialized: Message = serde_json::from_slice(&message.data).expect("failed to deserialize");
                        warn!("Got message: '{deserialized:#?}' with id: {message_id} from peer: {propagation_source}");
                    },
                    BehaviourEvent::Kad(kad_e) => { warn!("kad event: {kad_e:#?}"); },
                    _ => {warn!("unhandled behavior event: {beh_event:#?}");}

                }
            },
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                warn!("New connection to peer: {peer_id:#?}")
            }
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                warn!("Closed connection to peer: {peer_id:#?}")
            }
            event => {
                tracing::error!("Unhandled Event: {event:#?}");
            }
            //     SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
            //         for (peer_id, _multiaddr) in list {
            //             println!("mDNS discovered a new peer: {peer_id}");
            //             swarm.behaviour_mut().gossip.add_explicit_peer(&peer_id);
            //         }
            //     },
            //     SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
            //         for (peer_id, _multiaddr) in list {
            //             println!("mDNS discover peer has expired: {peer_id}");
            //             swarm.behaviour_mut().gossip.remove_explicit_peer(&peer_id);
            //         }
            //     },
            //     SwarmEvent::Behaviour(MyBehaviourEvent::Gossipsub(gossipsub::Event::Message {
            //         propagation_source: peer_id,
            //         message_id: id,
            //         message,
            //     })) => println!(
            //             "Got message: '{}' with id: {id} from peer: {peer_id}",
            //             String::from_utf8_lossy(&message.data),
            //         ),
            //     SwarmEvent::NewListenAddr { address, .. } => {
            //         println!("Local node is listening on {address}");
            //     }
            //     _ => {}
            }
        }
    }

    // loop {
    //     let evt = {
    //         tokio::select! {
    //             line = stdin.next_line() => Some(EventType::Input(line.expect("can get line").expect("can read line from stdin"))),
    //             event = swarm.next() => {
    //                 info!("Unhandled Swarm Event: {:?}", event);
    //                 None
    //             },
    //             response = response_rcv.recv() => Some(EventType::Response(response.expect("response exists"))),
    //         }
    //     };
    //     ...
    // }

    // loop {
    //     let evt = {
    //         tokio::select! {
    //             line = stdin.next_line() => Some(line.expect("can get line").expect("can read line from stdin")),
    //             event = swarm.next() => {
    //                 warn!("Unhandled Swarm Event: {:?}", event);
    //                 None
    //             },
    //             // response = response_rcv.recv() => Some(EventType::Response(response.expect("response exists"))),
    //         }
    //     };
    //
    //     if let Some(txt) = evt {
    //         warn!("Got from stdin: {txt}");
    //     }
    // }
    //     // loop {
    //     match swarm.select_next_some().await {
    //         SwarmEvent::NewListenAddr { address, .. } => warn!("Listening on {address:?}"),
    //         SwarmEvent::Behaviour(event) => warn!("EVENT: {event:?}"),
    //         SwarmEvent::ConnectionEstablished { peer_id, .. } => {
    //             warn!("New connection to peer: {peer_id:#?}")
    //         }
    //         SwarmEvent::ConnectionClosed { peer_id, .. } => {
    //             warn!("Closed connection to peer: {peer_id:#?}")
    //         }
    //         event => {
    //             tracing::error!("Unhandled Event: {event:#?}");
    //         }
    //     }
    // }
}
