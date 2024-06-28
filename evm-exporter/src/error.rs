use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    RedisError(#[from] redis::RedisError),

    #[error(transparent)]
    PostgresError(#[from] postgres::Error),

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

    #[error(transparent)]
    UTypeConvertError(#[from] uint::FromHexError),

    #[error(transparent)]
    HTypeConvertError(#[from] fixed_hash::rustc_hex::FromHexError),
}

pub type Result<T> = std::result::Result<T, Error>;
