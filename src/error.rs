use jacquard::{client::SessionStoreError, types::string::AtStrError};
use jacquard_identity::resolver::IdentityError;
use jacquard_oauth::error::OAuthError;
use thiserror::Error;

pub trait MapErrExt<T> {
    fn map_session_store_err(self) -> Result<T, SessionStoreError>;
}

impl<T> MapErrExt<T> for Result<T, keyring::Error> {
    fn map_session_store_err(self) -> Result<T, SessionStoreError> {
        self.map_err(|e| SessionStoreError::Other(Box::new(e)))
    }
}

#[derive(Debug, Error)]
pub enum OnyxError {
    #[error("auth store error: {0}")]
    AuthStore(String),

    #[error("session store error: {0}")]
    SessionStoreStore(#[from] SessionStoreError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("identity error: {0}")]
    Identity(#[from] IdentityError),

    #[error("oauth error: {0}")]
    OAuthError(#[from] OAuthError),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

impl From<AtStrError> for OnyxError {
    fn from(value: AtStrError) -> Self {
        Self::Other(Box::new(value))
    }
}

impl From<tokio::sync::TryLockError> for OnyxError {
    fn from(value: tokio::sync::TryLockError) -> Self {
        Self::Other(Box::new(value))
    }
}
