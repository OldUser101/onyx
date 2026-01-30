use jacquard::{
    client::{AgentError, SessionStoreError},
    error::ClientError,
    types::string::AtStrError,
};
use jacquard_identity::resolver::IdentityError;
use jacquard_oauth::error::OAuthError;
use thiserror::Error;

use crate::parser::ParserError;

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
    #[error("auth: {0}")]
    Auth(String),

    #[error("io: {0}")]
    Io(String),

    #[error("parse: {0}")]
    Parse(String),

    #[error("unknown: {0}")]
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

impl From<SessionStoreError> for OnyxError {
    fn from(err: SessionStoreError) -> Self {
        OnyxError::Auth(err.to_string())
    }
}

impl From<std::io::Error> for OnyxError {
    fn from(err: std::io::Error) -> Self {
        OnyxError::Io(err.to_string())
    }
}

impl From<serde_json::Error> for OnyxError {
    fn from(err: serde_json::Error) -> Self {
        OnyxError::Parse(err.to_string())
    }
}

impl From<IdentityError> for OnyxError {
    fn from(err: IdentityError) -> Self {
        OnyxError::Other(err.to_string().into())
    }
}

impl From<OAuthError> for OnyxError {
    fn from(err: OAuthError) -> Self {
        OnyxError::Auth(err.to_string())
    }
}

impl From<ClientError> for OnyxError {
    fn from(err: ClientError) -> Self {
        OnyxError::Other(err.to_string().into())
    }
}

impl From<AgentError> for OnyxError {
    fn from(err: AgentError) -> Self {
        OnyxError::Other(err.to_string().into())
    }
}

impl From<ParserError> for OnyxError {
    fn from(err: ParserError) -> Self {
        match err {
            ParserError::Io(e) => OnyxError::Io(e.to_string()),
            _ => OnyxError::Parse(err.to_string()),
        }
    }
}
