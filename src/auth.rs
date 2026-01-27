use anyhow::anyhow;
use jacquard::{
    client::{
        Agent, AgentSession, FileAuthStore, SessionStore, SessionStoreError, token::StoredSession,
    },
    identity::JacquardResolver,
    prelude::IdentityResolver,
    types::{
        did::Did,
        string::{AtStrError, Handle},
    },
};
use jacquard_identity::PublicResolver;
use jacquard_oauth::{
    atproto::AtprotoClientMetadata,
    authstore::ClientAuthStore,
    client::OAuthClient,
    loopback::LoopbackConfig,
    session::{ClientData, ClientSessionData},
};
use keyring::Entry;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    fmt::Display,
    hash::Hash,
    path::{Path, PathBuf},
};

use crate::{
    StoreMethod,
    error::{MapErrExt, OnyxError},
};

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

// An light adaptation of `jacquard::FileAuthStore` for keyrings
pub struct KeyringAuthStore(KeyringTokenStore);

impl KeyringAuthStore {
    pub fn new(service: String) -> Self {
        Self(KeyringTokenStore::new(service))
    }
}

impl jacquard_oauth::authstore::ClientAuthStore for KeyringAuthStore {
    async fn get_session(
        &self,
        did: &Did<'_>,
        session_id: &str,
    ) -> Result<Option<ClientSessionData<'_>>, SessionStoreError> {
        let key = format!("{}_{}", did, session_id);
        if let StoredSession::OAuth(session) = self
            .0
            .get(&key)
            .await
            .ok_or(SessionStoreError::Other("not found".into()))?
        {
            Ok(Some(session.into()))
        } else {
            Ok(None)
        }
    }

    async fn upsert_session(
        &self,
        session: ClientSessionData<'_>,
    ) -> Result<(), SessionStoreError> {
        let key = format!("{}_{}", session.account_did, session.session_id);
        self.0
            .set(key, StoredSession::OAuth(session.into()))
            .await?;
        Ok(())
    }

    async fn delete_session(
        &self,
        did: &Did<'_>,
        session_id: &str,
    ) -> Result<(), SessionStoreError> {
        let key = format!("{}_{}", did, session_id).to_string();
        let entry = Entry::new(&self.0.service, &key).map_session_store_err()?;

        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(SessionStoreError::Other(Box::new(e))),
        }
    }

    async fn get_auth_req_info(
        &self,
        state: &str,
    ) -> Result<Option<jacquard_oauth::session::AuthRequestData<'_>>, SessionStoreError> {
        let key = format!("authreq_{}", state);
        if let StoredSession::OAuthState(auth_req) = self
            .0
            .get(&key)
            .await
            .ok_or(SessionStoreError::Other("not found".into()))?
        {
            Ok(Some(auth_req.into()))
        } else {
            Ok(None)
        }
    }

    async fn save_auth_req_info(
        &self,
        auth_req_info: &jacquard_oauth::session::AuthRequestData<'_>,
    ) -> Result<(), SessionStoreError> {
        let key = format!("authreq_{}", auth_req_info.state);
        let state = auth_req_info
            .clone()
            .try_into()
            .map_err(|e| SessionStoreError::Other(Box::new(e)))?;
        self.0.set(key, StoredSession::OAuthState(state)).await?;
        Ok(())
    }

    async fn delete_auth_req_info(&self, state: &str) -> Result<(), SessionStoreError> {
        let key = format!("authreq_{}", state);
        let entry = Entry::new(&self.0.service, &key).map_session_store_err()?;

        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(SessionStoreError::Other(Box::new(e))),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthSession {
    pub did: String,
    pub session_id: String,
    pub store: StoreMethod,
}

pub struct AuthSessionStore {
    pub config_dir: PathBuf,
}

impl AuthSessionStore {
    fn try_new(config_dir: &Path) -> Result<Self, OnyxError> {
        if !config_dir.exists() {
            std::fs::create_dir_all(config_dir)?;
        }

        Ok(Self {
            config_dir: config_dir.to_owned(),
        })
    }

    fn get_session(&self) -> Result<Option<AuthSession>, OnyxError> {
        let session_path = self.config_dir.join("session.json");
        if !session_path.exists() {
            return Ok(None);
        }

        let session_str = std::fs::read_to_string(session_path)?;
        let session = serde_json::from_str(&session_str)?;
        Ok(Some(session))
    }

    fn set_session(&self, session: &AuthSession) -> Result<(), OnyxError> {
        let session_str = serde_json::to_string(session)?;
        let session_path = self.config_dir.join("session.json");
        std::fs::write(&session_path, &session_str)?;
        Ok(())
    }

    fn delete_session(&self) -> Result<(), OnyxError> {
        let session_path = self.config_dir.join("session.json");
        if !session_path.exists() {
            return Ok(());
        }

        std::fs::remove_file(&session_path)?;
        Ok(())
    }
}

pub struct Authenticator {
    pub service: String,
    pub config_dir: PathBuf,

    resolver: JacquardResolver,
    auth_store: AuthSessionStore,
}

impl Authenticator {
    pub fn try_new(service: &str, config_dir: &Path) -> Result<Self, OnyxError> {
        Ok(Self {
            service: service.to_owned(),
            config_dir: config_dir.to_owned(),
            resolver: PublicResolver::default(),
            auth_store: AuthSessionStore::try_new(config_dir)?,
        })
    }

    async fn resolve_did(&self, ident: &str) -> Result<Did<'_>, OnyxError> {
        if let Ok(did) = ident.parse() {
            return Ok(did);
        }

        let handle = Handle::new(ident)?;
        let did = self.resolver.resolve_handle(&handle).await?;
        Ok(did)
    }

    pub async fn login(
        &self,
        ident: &str,
        store: StoreMethod,
        password: Option<String>,
    ) -> Result<(), OnyxError> {
        println!("creating new session");

        match password {
            Some(pass) => self.login_app_password(ident, store, pass).await,
            None => self.login_oauth(ident, store).await,
        }
    }

    async fn login_app_password(
        &self,
        ident: &str,
        store: StoreMethod,
        password: String,
    ) -> Result<(), OnyxError> {
        let did = self.resolve_did(ident).await?;
        Ok(())
    }

    async fn login_oauth(&self, ident: &str, store_method: StoreMethod) -> Result<(), OnyxError> {
        let did = self.resolve_did(ident).await?;

        let client_data = ClientData {
            keyset: None,
            config: AtprotoClientMetadata::default_localhost(),
        };

        // There's probably a better way of doing this to avoid duplication,
        // but stores aren't dyn-compatible, and I couldn't be bothered
        if store_method == StoreMethod::Keyring {
            let store = KeyringAuthStore::new(self.service.clone());
            let oauth = OAuthClient::new(store, client_data);
            let session = oauth
                .login_with_local_server(&did, Default::default(), LoopbackConfig::default())
                .await?;

            let session_id = session.data.try_read()?.session_id.clone();
            let auth_session = AuthSession {
                did: did.to_string(),
                session_id: session_id.to_string(),
                store: store_method,
            };
            self.auth_store.set_session(&auth_session)?;
        } else if store_method == StoreMethod::File {
            let store = FileAuthStore::new(self.get_file_store());
            let oauth = OAuthClient::new(store, client_data);
            let session = oauth
                .login_with_local_server(&did, Default::default(), LoopbackConfig::default())
                .await?;

            let session_id = session.data.try_read()?.session_id.clone();
            let auth_session = AuthSession {
                did: did.to_string(),
                session_id: session_id.to_string(),
                store: store_method,
            };
            self.auth_store.set_session(&auth_session)?;
        }

        Ok(())
    }

    pub async fn restore(&self) -> Result<(), OnyxError> {
        let session = match self.auth_store.get_session()? {
            Some(s) => s,
            None => {
                return Err(OnyxError::AuthStore("no session to restore".to_string()));
            }
        };

        todo!("Implement restoration");

        Ok(())
    }

    pub async fn logout(&self) -> Result<(), OnyxError> {
        let session = match self.auth_store.get_session()? {
            Some(s) => s,
            None => {
                return Err(OnyxError::AuthStore("no session to logout".to_string()));
            }
        };

        let did = Did::new(&session.did)?;

        if session.store == StoreMethod::Keyring {
            let store = KeyringAuthStore::new(self.service.clone());
            store.delete_session(&did, &session.session_id).await?;
        } else if session.store == StoreMethod::File {
            let store = FileAuthStore::new(self.get_file_store());
            store.delete_session(&did, &session.session_id).await?;
        }

        self.auth_store.delete_session()
    }

    fn get_file_store(&self) -> PathBuf {
        self.config_dir.join("store.json")
    }
}
