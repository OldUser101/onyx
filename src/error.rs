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
    #[error("auth store error: {0}")]
    AuthStore(String),

    #[error("session store error: {0}")]
    SessionStore(String),

    #[error("io error: {0}")]
    Io(String),

    #[error("serde error: {0}")]
    Serde(String),

    #[error("identity error: {0}")]
    Identity(String),

    #[error("oauth error: {0}")]
    OAuthError(String),

    #[error("client error: {0}")]
    ClientError(String),

    #[error("agent error: {0}")]
    AgentError(String),

    #[error("parser error: {0}")]
    ParserError(String),

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

impl From<SessionStoreError> for OnyxError {
    fn from(err: SessionStoreError) -> Self {
        OnyxError::SessionStore(err.to_string())
    }
}

impl From<std::io::Error> for OnyxError {
    fn from(err: std::io::Error) -> Self {
        OnyxError::Io(err.to_string())
    }
}

impl From<serde_json::Error> for OnyxError {
    fn from(err: serde_json::Error) -> Self {
        OnyxError::Serde(err.to_string())
    }
}

impl From<IdentityError> for OnyxError {
    fn from(err: IdentityError) -> Self {
        OnyxError::Identity(err.to_string())
    }
}

impl From<OAuthError> for OnyxError {
    fn from(err: OAuthError) -> Self {
        OnyxError::OAuthError(err.to_string())
    }
}

impl From<ClientError> for OnyxError {
    fn from(err: ClientError) -> Self {
        OnyxError::ClientError(err.to_string())
    }
}

impl From<AgentError> for OnyxError {
    fn from(err: AgentError) -> Self {
        OnyxError::AgentError(err.to_string())
    }
}

impl From<ParserError> for OnyxError {
    fn from(err: ParserError) -> Self {
        OnyxError::ParserError(err.to_string())
    }
}
