pub mod methods;
pub mod socket;
use macros::RpcMessageParams;
use socket::Request;

use crate::MainResult;

#[derive(Debug, Copy, Clone, PartialEq)]
enum Namespace {
    Chain,
    Net,
    Client,
}

impl Namespace {
    const CHAIN: &str = "chain";
    const NET: &str = "net";
    const CLIENT: &str = "client";
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

trait RpcMessageParams {
    fn try_from_socket_request(req: &socket::Request) -> MainResult<Option<Self>>
    where
        Self: Sized,
    {
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
    fn method() -> &'static str;
    fn namespace() -> Namespace;
}

#[derive(RpcMessageParams)]
#[rpc_message(namespace = "chain")]
struct TestMessageParams {
    param1: String,
}

// #[test]
// fn derive() {
// }
//     assert_eq!(Foo::method(), 0);
