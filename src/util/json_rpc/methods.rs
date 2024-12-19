use super::*;
use crate::MainErr;
use ::macros::RpcRequest;
use serde::{Deserialize, Serialize};

enum RequestMethod {
    Test(GetPeerCountRequest),
}

impl TryFrom<socket::Request> for RequestMethod {
    type Error = MainErr;
    fn try_from(req: socket::Request) -> Result<Self, Self::Error> {
        if let Some(req) = GetPeerCountRequest::try_from_request(&req)? {
            return Ok(Self::Test(req));
        }
        Err("Could not get request".into())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPeerCountResponse {
    count: u32,
}

#[derive(RpcRequest, Debug, Clone)]
#[rpc_request(namespace = "net")]
pub struct GetPeerCountRequest {}
