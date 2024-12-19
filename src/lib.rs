pub mod blockchain;
pub mod node;
pub mod runtime;
pub mod telemetry;
pub mod util;

// these should be mapped to real models down the line
const MODEL_ID_0: &str = "model_0";
const MODEL_ID_1: &str = "model_1";

// static KEYS: LazyLock<Keypair> = LazyLock::new(|| Keypair::generate_ed25519());
// static PEER_ID: LazyLock<PeerId> = LazyLock::new(|| PeerId::from(KEYS.public()));
pub const CHAIN_TOPIC: &str = "chain_updates";

pub type MainErr = Box<dyn std::error::Error + Send + Sync + 'static>;
pub type MainResult<T> = std::result::Result<T, MainErr>;
