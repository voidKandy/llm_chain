use super::*;
use clap::{Parser, Subcommand};

/// Use the ETH JSON RPC as reference
/// https://ethereum.org/en/developers/docs/apis/json-rpc/#json-rpc-methods
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct UserInput {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Boot,
    Client,
}

impl<T> Node<T>
where
    T: NodeType,
    <<T as NodeType>::Behaviour as NetworkBehaviour>::ToSwarm: std::fmt::Debug,
    SwarmEvent<<<T as NodeType>::Behaviour as NetworkBehaviour>::ToSwarm>:
        Into<SwarmEvent<NodeBehaviourEvent>>,
{
    pub async fn handle_user_input(&mut self, line: String) -> MainResult<()> {
        match UserInput::try_parse_from(line.split_whitespace()) {
            Err(err) => {
                tracing::warn!("failed to parse user input: {line}");
                Ok(())
            }
            Ok(input) => {
                //
                Ok(())
            }
        }
    }
}
