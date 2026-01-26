use jacquard::client::SessionStoreError;

pub trait MapErrExt<T> {
    fn map_session_store_err(self) -> Result<T, SessionStoreError>;
}

impl<T> MapErrExt<T> for Result<T, keyring::Error> {
    fn map_session_store_err(self) -> Result<T, SessionStoreError> {
        self.map_err(|e| SessionStoreError::Other(Box::new(e)))
    }
}
