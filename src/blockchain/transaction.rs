use super::ChainResult;
use crate::MainResult;
use libp2p::identity::{Keypair, PublicKey};
use serde::{Deserialize, Serialize};
use sha3::{digest::core_api::CoreWrapper, Digest, Sha3_256, Sha3_256Core};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublicKeyBytes(Vec<u8>);

impl AsRef<Vec<u8>> for PublicKeyBytes {
    fn as_ref(&self) -> &Vec<u8> {
        &self.0
    }
}

impl From<PublicKey> for PublicKeyBytes {
    fn from(value: PublicKey) -> Self {
        Self(value.encode_protobuf())
    }
}

impl TryInto<PublicKey> for &PublicKeyBytes {
    type Error = libp2p::identity::DecodingError;
    fn try_into(self) -> Result<PublicKey, Self::Error> {
        PublicKey::try_decode_protobuf(&self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Transaction {
    pub(super) hash: String,
    pub(super) timestamp: String,
    pub(super) sender: PublicKeyBytes,
    pub(super) receiver: PublicKeyBytes,
    pub(super) tokens: f64,
    pub(super) inputs: Vec<Input>,
    pub(super) outputs: Vec<Output>,
    pub(super) signature: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Input {
    pub transaction_id: String,
    pub output_index: usize,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Output {
    pub receiver: PublicKeyBytes,
    pub amount: f64,
}

pub fn update_multiple<T>(hasher: &mut CoreWrapper<Sha3_256Core>, vec: &Vec<T>) -> MainResult<()>
where
    T: Serialize,
{
    for v in vec {
        let serialized = serde_json::to_string(&v)?;
        hasher.update(serialized);
    }
    Ok(())
}

impl Transaction {
    pub fn hash(&self) -> &str {
        &self.hash
    }
    pub fn tokens(&self) -> f64 {
        self.tokens
    }

    pub fn validate_hash(&self) -> bool {
        let mut hasher = Sha3_256::new();
        hasher.update(&self.timestamp);
        hasher.update(&self.sender.0);
        hasher.update(&self.receiver.0);
        hasher.update(self.tokens.to_string());
        update_multiple(&mut hasher, &self.inputs);
        update_multiple(&mut hasher, &self.outputs);
        let calculated_hash = format!("{:x}", hasher.finalize());
        self.hash == calculated_hash
    }

    pub fn sign(&mut self, keys: &Keypair) -> ChainResult<()> {
        if !self.validate_hash() {
            return Err(crate::chain::ChainError::HashInvalid);
        }
        if self.signature.is_some() {
            return Err(super::ChainError::SignatureExists);
        }
        let mut hasher = Sha3_256::new();
        hasher.update(&self.hash);
        hasher.update(&self.timestamp);
        hasher.update(self.tokens.to_string());
        hasher.update(&self.sender.0);
        hasher.update(&self.receiver.0);
        update_multiple(&mut hasher, &self.inputs);
        update_multiple(&mut hasher, &self.outputs);
        let transaction_digest = hasher.finalize();
        let signature = keys.sign(&transaction_digest)?;
        self.signature = Some(signature);
        Ok(())
    }

    /// returns None if the Tx is not signed, otherwise verifies signature
    pub fn verify_signature(&self) -> Option<bool> {
        self.signature.as_ref().and_then(|sig| {
            let mut hasher = Sha3_256::new();
            hasher.update(&self.hash);
            hasher.update(&self.timestamp);
            hasher.update(self.tokens.to_string());
            hasher.update(&self.sender.0);
            hasher.update(&self.receiver.0);
            update_multiple(&mut hasher, &self.inputs);
            update_multiple(&mut hasher, &self.outputs);
            let transaction_digest = hasher.finalize();
            let sender_public_key =
                TryInto::<PublicKey>::try_into(&self.sender).expect("failed to decode public key");
            Some(sender_public_key.verify(&transaction_digest, sig))
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::chain::ChainError;

    use super::*;
    use libp2p::identity;

    #[test]
    fn validate_hash_detects_tampered_fields() {
        let sender = identity::Keypair::generate_ed25519().public();
        let receiver = identity::Keypair::generate_ed25519().public();
        let timestamp = "2024-12-16T12:00:00Z".to_string();
        let inputs = String::from("input");
        let outputs = String::from("output");
        let tokens = 100.0;

        let mut transaction =
            Transaction::new(timestamp, sender, receiver, tokens, inputs, outputs);

        // Ensure the hash is valid initially.
        assert!(transaction.validate_hash());

        // Tamper with the transaction fields.
        transaction.tokens = 200.0;

        // The hash validation should now fail.
        assert!(!transaction.validate_hash());
    }

    #[test]
    fn sign_adds_valid_signature_and_prevents_resigning() {
        let keys = identity::Keypair::generate_ed25519();
        let sender = keys.public();
        let receiver = identity::Keypair::generate_ed25519().public();
        let timestamp = "2024-12-16T12:00:00Z".to_string();
        let inputs = String::from("input");
        let outputs = String::from("output");
        let tokens = 50.0;

        let mut transaction =
            Transaction::new(timestamp, sender, receiver, tokens, inputs, outputs);

        // Sign the transaction.
        assert!(transaction.sign(&keys).is_ok());

        // Ensure a signature is added.
        assert!(transaction.signature.is_some());

        // Attempt to re-sign the transaction, which should error.
        assert!(transaction.sign(&keys).is_err_and(|err| match err {
            ChainError::SignatureExists => true,
            _ => false,
        }))
    }

    #[test]
    fn verify_signature_correctly_validates_transaction() {
        let keys = identity::Keypair::generate_ed25519();
        let sender = keys.public();
        let receiver = identity::Keypair::generate_ed25519().public();
        let timestamp = "2024-12-16T12:00:00Z".to_string();
        let inputs = String::from("input");
        let outputs = String::from("output");
        let tokens = 75.0;

        let mut transaction =
            Transaction::new(timestamp, sender, receiver, tokens, inputs, outputs);

        // Sign the transaction.
        transaction.sign(&keys).unwrap();

        // Verify the signature is valid.
        assert_eq!(transaction.verify_signature(), Some(true));

        // Tamper with the transaction fields.
        transaction.tokens = 150.0;

        // The signature verification should now fail.
        assert_eq!(transaction.verify_signature(), Some(false));
    }
}
