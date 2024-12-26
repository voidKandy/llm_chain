use seraphic::{RpcNamespace, RpcRequest, RpcRequestWrapper};

#[derive(RpcNamespace, Copy, Clone, PartialEq, Eq)]
pub enum ClientNodeNamespace {
    Client,
}

#[derive(RpcRequestWrapper, Debug)]
pub enum ClientRequestWrapper {
    FindProvider(FindProviderRequest),
}

#[derive(RpcRequest, Debug, Clone, serde::Serialize, serde::Deserialize)]
#[rpc_request(namespace = "ClientNodeNamespace:client")]
pub struct FindProviderRequest;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FindProviderResponse {}
