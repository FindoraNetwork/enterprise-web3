use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    RedisError(#[from] redis::RedisError),

    #[error(transparent)]
    PostgresError(#[from] sqlx::Error),

    #[error(transparent)]
    FromHexError(#[from] hex::FromHexError),

    #[error(transparent)]
    FromStrRadixError(#[from] uint::FromStrRadixErr),

    #[error("Slice length mismatch")]
    LengthMismatch,

    #[error("Value not found")]
    ValueNotFound,

    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),

    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),

    #[error(transparent)]
    Libsecp256k1Error(#[from] libsecp256k1::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
