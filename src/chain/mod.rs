use libp2p::identity::SigningError;

pub mod block;
pub mod transaction;

pub type ChainResult<T> = Result<T, ChainError>;

#[derive(thiserror::Error, Debug)]
pub enum ChainError {
    #[error("libp2p failed to sign: {0:#?}")]
    Signing(#[from] SigningError),
    #[error("Hash validation failed")]
    HashInvalid,
    #[error("Tried to sign transaction which has already been signed")]
    SignatureExists,
}
