mod helpers;
use std::sync::LazyLock;

use helpers::TEST_TRACING;
use llm_chain::{
    chain::block::init_chain,
    node::{provider::ProviderNode, Node},
};

#[tokio::test]
async fn main() {
    LazyLock::force(&TEST_TRACING);
    let mut provider = Node::<ProviderNode>::init(None, init_chain()).unwrap();
    // return node.main_loop().await;
    //
}
