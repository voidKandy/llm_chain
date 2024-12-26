pub mod behaviour;
pub mod blockchain;
pub mod node;
pub mod runtime;
pub mod telemetry;
pub mod util;

// these should be mapped to real models down the line
const MODEL_ID_0: &str = "model_0";
const MODEL_ID_1: &str = "model_1";

pub type MainErr = Box<dyn std::error::Error + Send + Sync + 'static>;
pub type MainResult<T> = std::result::Result<T, MainErr>;
