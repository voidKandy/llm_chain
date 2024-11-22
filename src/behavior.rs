use chrono::Utc;
use libp2p::{
    gossipsub::{self, MessageAuthenticity},
    identify,
    identity::Keypair,
    kad::{self, store::MemoryStore},
    request_response::{self, ProtocolSupport},
    swarm::NetworkBehaviour,
    PeerId, StreamProtocol,
};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    io::Write,
};

#[derive(Debug, Hash, Deserialize, Serialize)]
pub struct CompletionReq {
    //shouldnt have to do this, make sure to remove
    /// Hash from the requestor's PeerID, current timestamp, model_id and prompt
    hash: String,
    pub model_id: String,
    pub prompt: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CompletionRes {
    // rq_hash: &'m str,
    // this should be encrypted eventually
    pub status: CompResStatus,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum CompResStatus {
    Working(String),
    Finished,
}

const IDENTIFY_ID: &str = "/ipfs/id/1.0.0";
#[derive(NetworkBehaviour)]
pub struct SysBehaviour {
    pub gossip: gossipsub::Behaviour,
    pub kad: kad::Behaviour<MemoryStore>,
    pub identify: identify::Behaviour,
    pub req_res: request_response::json::Behaviour<CompletionReq, CompletionRes>,
}

impl CompletionReq {
    pub fn new<'id>(originator: &'id PeerId, prompt: &str, model_id: &str) -> Self {
        let now = Utc::now().to_string();
        let record = format!("{}{}{}{}", now, originator, model_id, prompt);
        let mut hasher = Sha3_256::new();
        let _ = hasher
            .write(record.as_bytes())
            .expect("failed to write to hasher buffer");
        let hash_vec = hasher.finalize();
        let hash = String::from_utf8_lossy(&hash_vec).to_string();
        Self {
            hash,
            model_id: model_id.to_string(),
            prompt: prompt.to_string(),
        }
    }
}

impl SysBehaviour {
    pub fn new(peer_id: PeerId, key: Keypair) -> Self {
        let message_id_fn = |message: &gossipsub::Message| {
            let mut s = DefaultHasher::new();
            message.data.hash(&mut s);
            gossipsub::MessageId::from(s.finish().to_string())
        };
        let identify =
            identify::Behaviour::new(identify::Config::new(IDENTIFY_ID.to_string(), key.public()));

        let peer_store = MemoryStore::new(peer_id);
        let gossip_config = gossipsub::ConfigBuilder::default()
            .message_id_fn(message_id_fn)
            .build()
            .expect("failed to build gossip config");

        let gossip =
            gossipsub::Behaviour::new(MessageAuthenticity::Signed(key), gossip_config).unwrap();

        // fairly certain this protocol name is arbitrary
        let kad_config = kad::Config::new(StreamProtocol::new("/main"));

        // Good for debugging, by default this is set to 5 mins
        // kad_config.set_periodic_bootstrap_interval(Some(Duration::from_secs(10)));

        let kad = kad::Behaviour::<MemoryStore>::with_config(peer_id, peer_store, kad_config);

        let req_res = request_response::json::Behaviour::<CompletionReq, CompletionRes>::new(
            [(
                StreamProtocol::new("/completions_protocol/1.0.0"),
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
