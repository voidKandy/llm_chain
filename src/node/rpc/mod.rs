pub mod messages;
use super::*;
use crate::{
    blockchain::transaction::{mint::Mint, transfer::Transfer, UTXO},
    util::{
        json_rpc::RpcRequestWrapper,
        map_vec::{Contains, MapVec},
        PublicKeyBytes,
    },
};
pub use messages::*;

#[allow(private_interfaces)]
impl<T> Node<T>
where
    T: NodeType,
    <<T as NodeType>::Behaviour as NetworkBehaviour>::ToSwarm: std::fmt::Debug,
    SwarmEvent<<<T as NodeType>::Behaviour as NetworkBehaviour>::ToSwarm>:
        Into<SwarmEvent<NodeBehaviourEvent>>,
{
    pub async fn process_request_method(
        &mut self,
        req: RequestWrapper,
    ) -> MainResult<Result<serde_json::Value, socket::Error>> {
        tracing::warn!("processing: {req:#?}");
        match req {
            RequestWrapper::PeerCount(_) => {
                let count = self
                    .swarm
                    .behaviour_mut()
                    .as_mut()
                    .gossip
                    .all_peers()
                    .count() as u32;
                let response = GetPeerCountResponse { count };
                let json = serde_json::to_value(response)?;
                Ok(Ok(json))
            }
            RequestWrapper::GetBalance(_get_bal) => {
                let my_pub_key = PublicKeyBytes::from(self.keys.public());

                // should evevnutally use the given address, but for now will return local addr
                if let Some(top_block) = self.blockchain.peek() {
                    let transfers: &MapVec<String, Transfer> = (*top_block).get_ref();
                    let mut quantity = transfers.iter_vals().fold(0., |mut sum, t| {
                        let utxos: &MapVec<String, UTXO> = t.get_ref();
                        for v in utxos.iter_vals() {
                            let key: &PublicKeyBytes = v.get_ref();
                            if *key == my_pub_key {
                                sum += v.amount();
                            }
                        }
                        sum
                    });
                    let mint: &Mint = (*top_block).get_ref();
                    let utxos: &MapVec<PublicKeyBytes, UTXO> = mint.get_ref();
                    quantity += utxos
                        .get(&my_pub_key)
                        .and_then(|out| Some(out.amount()))
                        .unwrap_or(&0.0);
                    let response = GetBalanceResponse { quantity };
                    let json = serde_json::to_value(response)?;
                    return Ok(Ok(json));
                }
                Ok(Err(socket::Error::new_empty("1", "Empty chain")))
            }
        }
    }

    pub async fn handle_rpc_request(
        &mut self,
        req: socket::Request,
    ) -> MainResult<socket::Response> {
        let rpc_version = req.jsonrpc.clone();
        let req_id = req.id.clone();
        let method = RequestWrapper::try_from_rpc_req(req)?;
        let response = match self.process_request_method(method).await? {
            Ok(json) => socket::Response {
                jsonrpc: rpc_version,
                id: req_id,
                result: Some(json),
                error: None,
            },
            Err(err) => socket::Response {
                jsonrpc: rpc_version,
                id: req_id,
                result: None,
                error: Some(err),
            },
        };
        Ok(response)
    }
}
