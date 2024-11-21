use crate::chain::{block::Block, transaction::Transaction};
use tokio::io::{AsyncBufReadExt, BufReader, Lines, Stdin};

pub struct Node {
    pub typ: NodeType,
    ledger: Vec<Block>,
}

#[derive(Debug)]
pub enum NodeType {
    Client {
        stdin: Lines<BufReader<Stdin>>,
    },
    Validator {
        // vec might not be the best way to do this but it is fine for now
        tx_pool: Vec<Transaction>,
    },
    Provider,
}

impl NodeType {
    async fn try_read_stdin(&mut self) -> Option<String> {
        if let Self::Client { stdin, .. } = self {
            return stdin.next_line().await.unwrap();
        }
        None
    }
    pub fn is_validator(&self) -> bool {
        if let Self::Validator { .. } = self {
            return true;
        }
        false
    }
    pub fn is_client(&self) -> bool {
        if let Self::Client { .. } = self {
            return true;
        }
        false
    }
    pub fn is_provider(&self) -> bool {
        if let Self::Provider = self {
            return true;
        }
        false
    }
    pub fn new_client() -> Self {
        NodeType::Client {
            stdin: tokio::io::BufReader::new(tokio::io::stdin()).lines(),
        }
    }
}

impl Node {
    pub fn new(typ: NodeType, ledger: Vec<Block>) -> Node {
        Node { typ, ledger }
    }

    pub fn ledger_bytes(&self) -> serde_json::Result<Vec<u8>> {
        serde_json::to_vec(&self.ledger)
    }

    /// If other ledger is longer, replace mine with other.
    /// Returns whether or not ledger was replaced
    pub fn replace_ledger(&mut self, other: Vec<Block>) -> bool {
        let replace = other.len() > self.ledger.len();
        if replace {
            self.ledger = other;
        }

        replace
    }
}
