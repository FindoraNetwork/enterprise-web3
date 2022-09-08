use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    RedisError(#[from] redis::RedisError),
}

pub type Result<T> = std::result::Result<T, Error>;

