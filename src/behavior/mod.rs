pub mod gossip;
use gossip::{CompConnect, CompConnectConfirm};
use libp2p::{
    gossipsub::{self, MessageAuthenticity},
    identify,
    identity::Keypair,
    kad::{self, store::MemoryStore},
    request_response::{self, ProtocolSupport},
    swarm::NetworkBehaviour,
    StreamProtocol,
};
use std::hash::{DefaultHasher, Hash, Hasher};

const IDENTIFY_ID: &str = "/id/1.0.0";

#[derive(NetworkBehaviour)]
pub struct SysBehaviour {
    pub gossip: gossipsub::Behaviour,
    pub kad: kad::Behaviour<MemoryStore>,
    pub identify: identify::Behaviour,
    pub req_res: gossip::CompReqRes,
}

// impl CompletionReq {
//     pub fn new<'id>(originator: &'id PeerId, prompt: &str, model_id: &str) -> Self {
//         let now = Utc::now().to_string();
//         let record = format!("{}{}{}{}", now, originator, model_id, prompt);
//         let mut hasher = Sha3_256::new();
//         let _ = hasher
//             .write(record.as_bytes())
//             .expect("failed to write to hasher buffer");
//         let hash_vec = hasher.finalize();
//         let hash = String::from_utf8_lossy(&hash_vec).to_string();
//         Self {
//             hash,
//             model_id: model_id.to_string(),
//             prompt: prompt.to_string(),
//         }
//     }
// }

pub const KAD_PROTOCOL: StreamProtocol = StreamProtocol::new("/kademlia/1.0.0");
impl SysBehaviour {
    pub fn new(key: Keypair) -> Self {
        let message_id_fn = |message: &gossipsub::Message| {
            let mut s = DefaultHasher::new();
            message.data.hash(&mut s);
            gossipsub::MessageId::from(s.finish().to_string())
        };
        let identify =
            identify::Behaviour::new(identify::Config::new(IDENTIFY_ID.to_string(), key.public()));

        let gossip_config = gossipsub::ConfigBuilder::default()
            .message_id_fn(message_id_fn)
            .build()
            .expect("failed to build gossip config");

        let peer_store = MemoryStore::new(key.public().to_peer_id());
        // fairly certain this protocol name is arbitrary
        let kad_config = kad::Config::new(KAD_PROTOCOL);
        // Good for debugging, by default this is set to 5 mins
        // kad_config.set_periodic_bootstrap_interval(Some(Duration::from_secs(10)));

        let kad = kad::Behaviour::<MemoryStore>::with_config(
            key.public().to_peer_id(),
            peer_store,
            kad_config,
        );

        let gossip =
            gossipsub::Behaviour::new(MessageAuthenticity::Signed(key), gossip_config).unwrap();

        let req_res = request_response::json::Behaviour::<CompConnect, CompConnectConfirm>::new(
            [(
                StreamProtocol::new("/compreqres/1.0.0"),
                ProtocolSupport::Full,
            )],
            request_response::Config::default(),
        );

        SysBehaviour {
            gossip,
            kad,
            identify,
            req_res,
        }
    }
}
