use serde::{Deserialize, Serialize};

/// this will be coerced from the 'method' field of Request
/// Just like the eth JSON RPC API, the string will be structured as follows:
/// <namespace>_<method>
/// Which map to variants, and subtypes of this enum
/// I'm not entirely sure how to separate these subtypes, but I think this is a good place to start
#[derive(Debug, Clone, PartialEq)]
pub enum Method {
    Chain(ChainMethod),
    Net(NetMethod),
}

impl<'de> Deserialize for ChainMethod {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de> {
        deserializer.deserialize_string()
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChainMethod {
    GetTransaction,
}

#[derive(Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum NetMethod {}

mod tests {
    use super::Method;

    #[test]
    fn test_method_deserialize() {
        let test = "chain_getTransaction";
        let expected = Method::Chain(super::ChainMethod::GetTransaction);

        let str = serde_json::to_string(&super::ChainMethod::GetTransaction).unwrap();
        println!("string: {str}");

        let method = Method::try_from(test).unwrap();
        assert_eq!(expected, method);
    }
}
