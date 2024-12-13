pub mod client;
pub mod provider;
pub mod validator;
use crate::{
    behavior::{
        gossip::{ProvisionBid, SysTopic},
        SysBehaviour, SysBehaviourEvent, KAD_PROTOCOL,
    },
    chain::{
        block::{Block, Blockchain},
        transaction::CompletedTransaction,
    },
    MainResult, CHAIN_TOPIC,
};
use futures::StreamExt;
use http_body_util::BodyExt;
use hyper::{
    body::{Body, Bytes},
    service::{self, service_fn},
    Request, Response,
};
use libp2p::{
    gossipsub::{self, Message, TopicHash},
    identify,
    identity::Keypair,
    swarm::SwarmEvent,
    Multiaddr, PeerId, Swarm,
};
use sha3::{Digest, Sha3_256};
use std::{
    convert::Infallible,
    fmt::Debug,
    io::{BufReader, Read, Write},
    net::SocketAddr,
    ops::Add,
    time::{Duration, Instant},
};
use tracing::warn;

pub struct Node<T> {
    // may not need to be here, as they are acceisble through swarm
    keys: Keypair,
    swarm: Swarm<SysBehaviour>,
    // stdin: Lines<BufReader<Stdin>>,
    // this should maybe be changed to be general
    // Either way this is to provide a JSON interface
    tcp_listener: tokio::net::TcpListener,
    typ: T,
    // ledger: Vec<Block>,
    // should_publish_ledger: bool,
}

#[derive(Clone)]
pub struct TokioExecutor;

impl<F> hyper::rt::Executor<F> for TokioExecutor
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    fn execute(&self, fut: F) {
        tokio::task::spawn(fut);
    }
}

// VERY BAD!
#[derive(Debug)]
pub struct Wallet {
    id: String,
    pub balance: f64,
    // Vector of transaction Hashes
    transactions: Vec<String>,
}

impl Wallet {
    pub fn new() -> Self {
        let now = Instant::now();
        let mut hasher = Sha3_256::new();
        let _ = hasher
            .write(format!("{now:#?}").as_bytes())
            .expect("failed to write to hasher buffer");
        let hash_vec = hasher.finalize();
        let id = String::from_utf8_lossy(&hash_vec).to_string();
        Self {
            id,
            // BAD! Change this in prod to 0
            balance: 500.,
            transactions: vec![],
        }
    }
    pub fn push_tx(&mut self, tx: &CompletedTransaction) {
        self.transactions.push(tx.hash().to_string());
    }
}

#[allow(async_fn_in_trait)]
pub trait NodeType<'w>: Sized + Debug {
    // fn wallet_val(&'w mut self) -> &'w mut Wallet;
    //
    // fn publish_wallet_adjustment(
    //     &'w mut self,
    //     swarm: &mut Swarm<SysBehaviour>,
    //     bid: ProvisionBid,
    //     // f: impl FnOnce(&'w mut Wallet),
    // ) {
    //     self.wallet_val()
    //         .balance
    //         .add(match &bid.peer == swarm.local_peer_id() {
    //             true => bid.bid,
    //             false => bid.bid * -1.0,
    //         });
    //
    //     let _ = swarm
    //         .behaviour_mut()
    //         .gossip
    //         .publish(
    //             SysTopic::Completed.publish(),
    //             serde_json::to_vec(&bid).expect("failed to serialize bid"),
    //         )
    //         .expect("failed to publish transaction");
    // }
    fn init(swarm: &mut Swarm<SysBehaviour>) -> MainResult<Self>;

    async fn loop_logic(node: &mut Node<Self>) -> MainResult<()> {
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
        tracing::warn!("in default handle swarm");
        match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                warn!("Listening on {address:?}");
                // node.swarm
                //     .add_peer_address(*node.swarm.local_peer_id(), address);
                let local_id = node.swarm.local_peer_id().clone();
                node.swarm
                    .behaviour_mut()
                    .kad
                    .add_address(&local_id, address);
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
                // if node.replace_ledger(update) {
                //     warn!("replaced node's ledger");
                // } else {
                //     warn!("did not replace node's ledger");
                // }
            }

            SwarmEvent::Behaviour(SysBehaviourEvent::Identify(identify::Event::Received {
                peer_id,
                info,
                ..
            })) if info.protocols.iter().any(|p| *p == KAD_PROTOCOL) => {
                for addr in info.listen_addrs {
                    node.swarm.behaviour_mut().kad.add_address(&peer_id, addr);
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

async fn handle_request(
    req: Request<hyper::body::Incoming>,
) -> Result<Response<http_body_util::Full<hyper::body::Bytes>>, Infallible> {
    warn!("inside handle req");
    let headers = req.headers();
    // default to JSON
    let content_type = headers
        .get("Content-Type")
        .and_then(|h| Some(h.to_str().unwrap().to_lowercase()))
        .unwrap_or("application/json".to_string());

    match content_type.as_str() {
        "application/json" => {
            warn!("content type header: {content_type:#?}");
            let stream = req.into_data_stream();
            let bytes: Vec<u8> = stream
                .fold(Vec::<u8>::new(), |mut acc, bytes| async {
                    bytes.unwrap().bytes().for_each(|b| {
                        acc.push(b.unwrap());
                    });
                    acc
                })
                .await;
            let req: serde_json::Value =
                serde_json::from_slice(bytes.as_slice()).expect("failed to serialize body");
            warn!("request: {req:#?}");

            Ok(Response::new(http_body_util::Full::new(
                hyper::body::Bytes::from("Hello, Rust HTTP Server!"),
            )))
        }

        other => Ok(Response::new(http_body_util::Full::new(
            hyper::body::Bytes::from(format!("server does not support {other} content type")),
        ))),
    }
}

impl<'w, T> Node<T>
where
    T: NodeType<'w>,
{
    /// Create a node and start listening
    pub async fn init(
        dial_address: Option<Multiaddr>,
        // blockchain: Blockchain,
    ) -> MainResult<Node<T>> {
        let keys = Keypair::generate_ed25519();
        let local_id = PeerId::from(keys.public());
        let mut swarm = libp2p::SwarmBuilder::with_existing_identity(keys.clone())
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

        let addr = SocketAddr::from(([127, 0, 0, 1], 2345));
        let tcp_listener = tokio::net::TcpListener::bind(addr).await?;

        // let make_svc = hyper::service::service_fn(|req| async {
        //     handle_request
        // });

        // let server = hyper::server::conn::http2::Connection::from(addr);

        // if let Err(e) = server.await {
        //     eprintln!("server error: {}", e);
        // }

        swarm.listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse()?)?;
        // swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

        if let Some(addr) = dial_address {
            warn!("Dialing {addr}");
            swarm.dial(addr)?;
        }

        Ok(Node {
            keys,
            swarm,
            typ,
            tcp_listener,
            // ledger: blockchain,
            // should_publish_ledger: true,
        })
    }

    pub async fn main_loop(&mut self) -> MainResult<()> {
        loop {
            let (stream, _) = self
                .tcp_listener
                .accept()
                .await
                .expect("failed to accept incoming");

            let io = hyper_util::rt::TokioIo::new(stream);

            // Spin up a new task in Tokio so we can continue to listen for new TCP connection on the
            // current task without waiting for the processing of the HTTP/2 connection we just received
            // to finish
            tokio::task::spawn(async move {
                // Handle the connection from the client using HTTP/2 with an executor and pass any
                // HTTP requests received on that connection to the `hello` function
                if let Err(err) = hyper::server::conn::http2::Builder::new(TokioExecutor)
                    .serve_connection(io, service_fn(handle_request))
                    .await
                {
                    tracing::error!("Error serving connection: {}", err);
                }
            });

            // Self::handle_local_connection(stream);
        }
        // Ok(())
        // let chain_topic = TopicHash::from_raw(CHAIN_TOPIC);
        // loop {
        // if self.should_publish_ledger {
        //     let ledger = self.ledger_bytes()?;
        //     let _ = self
        //         .swarm
        //         .behaviour_mut()
        //         .gossip
        //         .publish(chain_topic.clone(), ledger);
        //     self.should_publish_ledger = false;
        // }
        // T::loop_logic(self).await?;
        // }
    }

    // fn handle_local_connection(mut stream: tokio::net::TcpStream) {
    //     let buf_reader = BufReader::new(&stream);
    // let mut buf = String::new();
    // buf_reader.read_to_string(&mut buf);

    // let json_req: serde_json::Value = serde_json::from_reader(buf_reader).unwrap();
    // let http_request: Vec<_> = buf_reader
    //     .lines()
    //     .map(|result| result.unwrap())
    //     .take_while(|line| !line.is_empty())
    //     .collect();

    //     warn!("Request: {json_req:#?}");
    // }

    // fn ledger_bytes(&self) -> serde_json::Result<Vec<u8>> {
    //     serde_json::to_vec(&self.ledger)
    // }

    // If other ledger is longer, replace mine with other.
    // Returns whether or not ledger was replaced
    // pub fn replace_ledger(&mut self, other: Vec<Block>) -> bool {
    //     let replace = other.len() > self.ledger.len();
    //     if replace {
    //         self.ledger = other;
    //         self.should_publish_ledger = true;
    //     }
    //     replace
    // }
}
