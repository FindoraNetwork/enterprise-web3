use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    RedisError(#[from] redis::RedisError),

    #[error(transparent)]
    FromHexError(#[from] hex::FromHexError),

    #[error(transparent)]
    FromStrRadixError(#[from] uint::FromStrRadixErr),

    #[error("Slice length mismatch")]
    LengthMismatch,

    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),

    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),
}

pub type Result<T> = std::result::Result<T, Error>;
