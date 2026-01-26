use jacquard::client::{SessionStore, SessionStoreError};
use keyring::Entry;
use serde::{Serialize, de::DeserializeOwned};
use std::{fmt::Display, hash::Hash};

use crate::error::MapErrExt;

#[derive(Clone, Debug)]
pub struct KeyringTokenStore {
    pub service: String,
}

impl KeyringTokenStore {
    pub fn new(service: String) -> Self {
        Self { service }
    }
}

impl<K: Send + Sync + Hash + Eq + Display, T: Send + Sync + Clone + Serialize + DeserializeOwned>
    SessionStore<K, T> for KeyringTokenStore
{
    async fn get(&self, key: &K) -> Option<T> {
        let key_string = key.to_string();
        let entry = Entry::new(&self.service, &key_string).ok()?;
        let value = entry.get_password().ok()?;
        serde_json::from_str(&value).ok()
    }

    async fn set(&self, key: K, session: T) -> Result<(), SessionStoreError> {
        let key_string = key.to_string();
        let entry = Entry::new(&self.service, &key_string).map_session_store_err()?;
        let value = serde_json::to_string(&session)?;
        entry.set_password(&value).map_session_store_err()?;
        Ok(())
    }

    async fn del(&self, key: &K) -> Result<(), SessionStoreError> {
        let key_string = key.to_string();
        let entry = Entry::new(&self.service, &key_string).map_session_store_err()?;
        entry.delete_credential().map_session_store_err()?;
        Ok(())
    }
}
