use crate::util::json_rpc::{Namespace, RpcRequest, RpcResponse, TryFromSocketRequest};
use ::macros::RpcRequest;
use serde::{Deserialize, Serialize};

use super::*;

#[derive(Debug)]
pub enum RequestMethod {
    PeerCount(GetPeerCountRequest),
}

impl RequestMethod {
    pub fn into_socket_request(self, id: u32, jsonrpc: &str) -> socket::Request {
        match self {
            Self::PeerCount(rq) => rq.into_socket_request(id, jsonrpc).unwrap(),
        }
    }
}

impl TryFromSocketRequest for RequestMethod {
    fn try_from_socket_req(req: socket::Request) -> MainResult<Self> {
        if let Some(req) = GetPeerCountRequest::try_from_request(&req)? {
            return Ok(Self::PeerCount(req));
        }
        Err("Could not get request".into())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPeerCountResponse {
    count: u32,
}

#[derive(RpcRequest, Debug, Clone, Serialize, Deserialize)]
#[rpc_request(namespace = "net")]
pub struct GetPeerCountRequest;

#[allow(private_interfaces)]
impl<T> Node<T>
where
    T: NodeType,
    <<T as NodeType>::Behaviour as NetworkBehaviour>::ToSwarm: std::fmt::Debug,
    SwarmEvent<<<T as NodeType>::Behaviour as NetworkBehaviour>::ToSwarm>:
        Into<SwarmEvent<NodeBehaviourEvent>>,
{
    pub async fn process_request_method(
        &mut self,
        req_meth: RequestMethod,
    ) -> MainResult<Result<serde_json::Value, socket::Error>> {
        tracing::warn!("processing: {req_meth:#?}");
        match req_meth {
            RequestMethod::PeerCount(_) => {
                let count = T::Behaviour::shared(self.swarm.behaviour_mut())
                    .gossip
                    .all_peers()
                    .count() as u32;
                let response = GetPeerCountResponse { count };
                let json = serde_json::to_value(response)?;
                Ok(Ok(json))
            }
        }
    }

    pub async fn handle_rpc_request(
        &mut self,
        req: socket::Request,
    ) -> MainResult<socket::Response> {
        let rpc_version = req.jsonrpc.clone();
        let req_id = req.id.clone();
        let method = RequestMethod::try_from_socket_req(req)?;
        let response = match self.process_request_method(method).await? {
            Ok(json) => socket::Response {
                jsonrpc: rpc_version,
                id: req_id,
                result: Some(json),
                error: None,
            },
            Err(err) => socket::Response {
                jsonrpc: rpc_version,
                id: req_id,
                result: None,
                error: Some(err),
            },
        };
        Ok(response)
    }
}
