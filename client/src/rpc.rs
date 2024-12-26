use seraphic::{RpcNamespace, RpcRequest, RpcRequestWrapper};

#[derive(RpcNamespace, Copy, Clone, PartialEq, Eq)]
pub enum ClientNodeNamespace {
    Client,
}

#[derive(RpcRequestWrapper, Debug)]
pub enum ClientRequestWrapper {
    StartAuction(StartAuctionRequest),
}

#[derive(RpcRequest, Debug, Clone, serde::Serialize, serde::Deserialize)]
#[rpc_request(namespace = "ClientNodeNamespace:client")]
pub struct StartAuctionRequest;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StartAuctionResponse {
    pub started: bool,
}
