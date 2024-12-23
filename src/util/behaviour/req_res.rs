use serde::{Deserialize, Serialize};

pub type NetworkReqRes = libp2p::request_response::json::Behaviour<NetworkRequest, NetworkResponse>;

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub enum NetworkRequest {
    /// Client requests server starts listening on stream
    OpenStream,
    // Chain,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub enum NetworkResponse {
    /// provider lets client know that it has started listening,
    OpenStreamAck,
    // Chain(Blockchain),
}
