pub mod error;
pub mod socket;
pub mod thread;
use crate::MainResult;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Namespace {
    Chain,
    Net,
    Client,
}

impl Namespace {
    const CHAIN: &str = "chain";
    const NET: &str = "net";
    const CLIENT: &str = "client";
}

impl AsRef<str> for Namespace {
    fn as_ref(&self) -> &str {
        match self {
            Self::Chain => Self::CHAIN,
            Self::Net => Self::NET,
            Self::Client => Self::CLIENT,
        }
    }
}

impl<'a> TryFrom<&'a str> for Namespace {
    type Error = std::io::Error;
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        match value {
            Self::CHAIN => Ok(Self::Chain),
            Self::NET => Ok(Self::Net),
            Self::CLIENT => Ok(Self::Client),
            _ => Err(std::io::Error::other(format!(
                "{value} is invalid namespace string"
            ))),
        }
    }
}

pub trait TryFromSocketRequest {
    fn try_from_socket_req(req: socket::Request) -> MainResult<Self>
    where
        Self: Sized;
}

pub trait RpcResponse:
    std::fmt::Debug + Clone + serde::Serialize + for<'de> serde::Deserialize<'de>
{
}

pub trait RpcRequest:
    std::fmt::Debug + Clone + serde::Serialize + for<'de> serde::Deserialize<'de>
{
    type Response: RpcResponse;
    fn method() -> &'static str;
    fn namespace() -> Namespace;
    fn into_socket_request(&self, id: u32, jsonrpc: &str) -> MainResult<socket::Request> {
        let params = serde_json::to_value(&self)?;
        let method = format!("{}_{}", Self::namespace().as_ref(), Self::method());
        Ok(socket::Request {
            jsonrpc: jsonrpc.to_string(),
            method,
            params,
            id: format!("{id}"),
        })
    }
    fn try_from_request(req: &socket::Request) -> MainResult<Option<Self>> {
        if let Some((namespace_str, method_str)) = req.method.split_once('_') {
            let namespace = Namespace::try_from(namespace_str).unwrap();
            if namespace != Self::namespace() || method_str != Self::method() {
                return Ok(None);
            }

            return Self::try_from_json(&req.params).and_then(|me| Ok(Some(me)));
        }
        Ok(None)
    }
    fn try_from_json(json: &serde_json::Value) -> MainResult<Self>
    where
        Self: Sized;
}
