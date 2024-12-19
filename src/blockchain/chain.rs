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
///
pub const BOOT_NODE_KEY_PATH: &str = "boot.key";
pub const BOOT_NODE_PEER_ID: &str = "12D3KooWCwDGQ5jED2DCkdjLpfitvBr6KMDW3VkFLMxE4f67vUen";
pub const BOOT_NODE_LOCAL_ADDR: &str = "/ip4/127.0.0.1/udp/62649/quic-v1";
pub const BOOT_NODE_LISTEN_ADDR: &str = "/ip4/0.0.0.0/udp/62649/quic-v1";

pub const BOOT_NODE_KEYPAIR: LazyLock<Keypair> = LazyLock::new(|| {
    let mut bytes = std::fs::read(BOOT_NODE_KEY_PATH).unwrap();
    Keypair::ed25519_from_bytes(&mut bytes).expect("failed to get keypair from boot.key bytes")
});

// pub fn new() -> Blockchain {
//     let block = LazyLock::force(&GENESIS_BLOCK).clone();
//     let mut chain = Blockchain {
//         data: MapVec::new(),
//     };
//     chain.data.push(block);
//     chain
// }

// pub fn validate(&self) -> bool {
//     self.data.iter_vals().all(|b| b.validate())
// }

// pub fn get_output_amt(&self, input: &Input) -> Option<f64> {
// self.tx_lookup
//     .get(&input.transaction_id)
//     .and_then(|(block_idx, tx_idx)| {
//         Some(self.data[*block_idx].transactions[*tx_idx].tokens())
//     })
// }

// pub fn new_transaction(
//     &self,
//     timestamp: String,
//     sender: PublicKey,
//     receiver: PublicKey,
//     tokens: f64,
//     inputs: Vec<Input>,
//     // outputs: Vec<Output>,
// ) -> Transaction {
//     let sender: PublicKeyBytes = sender.into();
//     let receiver: PublicKeyBytes = receiver.into();
//
//     // Calculate total input value (sum of all UTXOs referenced by inputs)
//     let total_input_value = inputs.iter().fold(0., |sum, input| {
//         sum + self
//             .get_output_amt(input)
//             .expect("input points to non-existant output")
//     });
//
//     // Determine the change (if any)
//     let change = total_input_value - tokens;
//
//     // Create the outputs (receiver + change)
//     let mut outputs = vec![Output {
//         receiver: receiver.clone(),
//         amount: tokens,
//     }];
//
//     if change > 0.0 {
//         outputs.push(Output {
//             receiver: sender.clone(), // The sender receives the change
//             amount: change,
//         });
//     }
//
//     let mut hasher = Sha3_256::new();
//     hasher.update(&timestamp);
//     hasher.update(&sender.as_ref());
//     hasher.update(&receiver.as_ref());
//     hasher.update(tokens.to_string());
//     update_multiple(&mut hasher, &inputs).unwrap();
//     // update_multiple(&mut hasher, &outputs);
//     let hash = format!("{:x}", hasher.finalize());
//
//     Transaction {
//         hash,
//         timestamp,
//         sender,
//         receiver,
//         tokens,
//         inputs,
//         outputs,
//         signature: None,
//     }
// }
