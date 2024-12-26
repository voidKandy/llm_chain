use crate::MainResult;

use seraphic::{socket, RpcNamespace, RpcRequest, RpcRequestWrapper, RpcResponse};
use serde::{Deserialize, Serialize};

#[derive(RpcNamespace, Debug, Copy, Clone, PartialEq)]
pub enum Namespace {
    Chain,
    Net,
    Client,
}

#[derive(RpcRequestWrapper, Debug)]
pub enum RequestWrapper {
    PeerCount(GetPeerCountRequest),
    GetBalance(GetBalanceRequest),
}

#[derive(RpcRequest, Debug, Clone, Serialize, Deserialize)]
#[rpc_request(namespace = "Namespace:net")]
pub struct GetPeerCountRequest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPeerCountResponse {
    pub count: u32,
}

#[derive(RpcRequest, Debug, Clone, Serialize, Deserialize)]
#[rpc_request(namespace = "Namespace:chain")]
pub struct GetBalanceRequest {
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBalanceResponse {
    pub quantity: f64,
}
