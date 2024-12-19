pub mod methods;
pub mod socket;
use macros::{RpcMessage, RpcMessageParams};

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
