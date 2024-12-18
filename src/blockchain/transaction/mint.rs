use crate::util::{hash::Hash, map_vec::MapVec, now_timestamp_string, PublicKeyBytes};
use libp2p::identity::PublicKey;
use serde::{Deserialize, Serialize};
use sha3::Digest;

use super::{transfer::Transfer, UTXO};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Mint {
    pub(super) hash: String,
    pub(super) timestamp: String,
    pub(super) outputs: MapVec<PublicKeyBytes, super::UTXO>,
}

pub struct Fields<'h> {
    timestamp: &'h str,
    outputs: &'h [super::UTXO],
}

impl<'h> From<&'h Mint> for Fields<'h> {
    fn from(value: &'h Mint) -> Self {
        Self {
            timestamp: &value.timestamp,
            outputs: value.outputs.as_ref(),
        }
    }
}

impl<'h> Hash<'h> for Mint {
    type Fields = Fields<'h>;
    fn hash_ref(&self) -> &str {
        &self.hash
    }
    fn hash_fields(fields: Self::Fields) -> sha3::digest::Output<crate::util::hash::Hasher> {
        let mut hasher = Self::hasher();
        hasher.update(fields.timestamp);
        Self::update_multiple(&mut hasher, fields.outputs);
        hasher.finalize()
    }
}

const MINT_INCENTIVE_TOTAL: f64 = 9999.0;
/// the amount of the mint incentive to divvy up between providers who contributed to the block
const PROVIDERS_POOL_PORTION: f64 = 0.15;
impl Mint {
    pub fn new(transfers: impl AsRef<[Transfer]>, miner_key: PublicKey) -> Self {
        // do some work to get all providers and percents from transfers
        let all_providers_and_percents: Vec<(&PublicKeyBytes, f64)> = vec![];
        let percent_sum = all_providers_and_percents
            .iter()
            .fold(0., |sum, (_, p)| sum + p);
        if percent_sum != 1.0 {
            tracing::warn!("expected all providers percents to sum to 1.0 got {percent_sum:#?}");
        }

        let providers_pool = MINT_INCENTIVE_TOTAL * PROVIDERS_POOL_PORTION;
        let miner_amt = MINT_INCENTIVE_TOTAL - providers_pool;

        let miner_utxo = UTXO::new(miner_amt, miner_key);
        let mut outputs = vec![miner_utxo];

        all_providers_and_percents
            .into_iter()
            .for_each(|(id, perc)| {
                let amt = providers_pool * perc;
                outputs.push(UTXO::new(amt, id.clone()));
            });

        let timestamp = now_timestamp_string();

        let fields = Fields {
            timestamp: &timestamp,
            outputs: &outputs,
        };
        let hash = Self::output_to_string(Self::hash_fields(fields));

        Self {
            hash,
            timestamp,
            outputs: outputs.into(),
        }
    }
}
