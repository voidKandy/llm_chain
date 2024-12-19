pub mod error;
pub mod socket;
pub mod thread;
use crate::MainResult;

pub trait RpcNamespace: PartialEq + Copy {
    fn as_str(&self) -> &str;
    fn try_from_str(str: &str) -> Option<Self>
    where
        Self: Sized;
}

pub trait SocketRequestWrapper {
    fn into_socket_request(self, id: u32, jsonrpc: &str) -> socket::Request
    where
        Self: Sized;
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
    type Namespace: RpcNamespace;
    fn method() -> &'static str;
    fn namespace() -> Self::Namespace;
    fn into_socket_request(&self, id: u32, jsonrpc: &str) -> MainResult<socket::Request> {
        let params = serde_json::to_value(&self)?;
        let method = format!("{}_{}", Self::namespace().as_str(), Self::method());
        Ok(socket::Request {
            jsonrpc: jsonrpc.to_string(),
            method,
            params,
            id: format!("{id}"),
        })
    }
    fn try_from_request(req: &socket::Request) -> MainResult<Option<Self>> {
        if let Some((namespace_str, method_str)) = req.method.split_once('_') {
            let namespace = Self::Namespace::try_from_str(namespace_str).unwrap();
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
