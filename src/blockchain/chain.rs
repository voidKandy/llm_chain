use super::block::Block;
use crate::util::map_vec::MapVec;
use libp2p::identity::Keypair;
use std::sync::LazyLock;

pub type Blockchain = MapVec<String, Block>;
const GENESIS_BLOCK: LazyLock<Block> = LazyLock::new(|| {
    let k = BOOT_NODE_KEYPAIR;
    let keys = LazyLock::force(&k);
    // the need to use keys twice here is a little concerning.. might be fine tho idk
    let block = Block::new_unsigned(0, 0, String::new(), vec![], keys.public());
    block.sign(keys).expect("failed to sign block")
});

pub fn init_blockchain() -> Blockchain {
    Blockchain::from(vec![LazyLock::force(&GENESIS_BLOCK).to_owned()])
}

/// boot node private key in boot.key, which was generated with
/// ```shell
/// head -c 32 /dev/urandom > boot.key
/// ```
/// These are just random
pub const BOOT_NODE_KEY: &[u8; 32] = &[
    212, 60, 214, 85, 84, 109, 59, 48, 212, 32, 49, 55, 254, 81, 225, 32, 100, 66, 43, 210, 78, 57,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];
pub const BOOT_NODE_PEER_ID: &str = "12D3KooWCwDGQ5jED2DCkdjLpfitvBr6KMDW3VkFLMxE4f67vUen";
pub const BOOT_NODE_LOCAL_ADDR: &str = "/ip4/127.0.0.1/udp/62649/quic-v1";
pub const BOOT_NODE_LISTEN_ADDR: &str = "/ip4/0.0.0.0/udp/62649/quic-v1";

pub const BOOT_NODE_KEYPAIR: LazyLock<Keypair> = LazyLock::new(|| {
    let mut bytes = BOOT_NODE_KEY.to_vec();

    Keypair::ed25519_from_bytes(&mut bytes).expect("failed to get keypair from boot.key bytes")
});
