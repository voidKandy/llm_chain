use crate::{
    util::json_rpc::{Namespace, RpcRequest, RpcResponse},
    MainResult,
};

use ::macros::RpcRequest;
use serde::{Deserialize, Serialize};

#[derive(RpcRequest, Debug, Clone, Serialize, Deserialize)]
#[rpc_request(namespace = "net")]
pub struct GetPeerCountRequest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPeerCountResponse {
    pub count: u32,
}

#[derive(RpcRequest, Debug, Clone, Serialize, Deserialize)]
#[rpc_request(namespace = "chain")]
pub struct GetBalanceRequest {
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBalanceResponse {
    pub quantity: f64,
}
