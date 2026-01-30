use jacquard::{
    CowStr, IntoStatic,
    client::{
        AgentSession, AtpSession, FileAuthStore, SessionStore, SessionStoreError,
        credential_session::{CredentialSession, SessionKey},
        token::StoredSession,
    },
    error::{ClientError, XrpcResult},
    identity::JacquardResolver,
    prelude::{HttpClient, IdentityResolver},
    types::{did::Did, string::Handle},
    xrpc::{XrpcClient, XrpcRequest, XrpcResponse},
};
use jacquard_identity::PublicResolver;
use jacquard_oauth::{
    atproto::AtprotoClientMetadata,
    authstore::ClientAuthStore,
    client::{OAuthClient, OAuthSession},
    loopback::LoopbackConfig,
    session::{ClientData, ClientSessionData},
};
use keyring::Entry;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    fmt::Display,
    hash::Hash,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    StoreMethod,
    error::{MapErrExt, OnyxError},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredPasswordSession {
    access_jwt: String,
    refresh_jwt: String,
    did: String,
    session_id: String,
    handle: String,
}

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

impl SessionStore<SessionKey, AtpSession> for KeyringAuthStore {
    async fn get(&self, key: &SessionKey) -> Option<AtpSession> {
        let key_str = format!("{}_{}", key.0, key.1);
        if let Some(stored) =
            SessionStore::<String, StoredPasswordSession>::get(&self.0, &key_str).await
        {
            Some(AtpSession {
                access_jwt: stored.access_jwt.into(),
                refresh_jwt: stored.refresh_jwt.into(),
                did: stored.did.into(),
                handle: stored.handle.into(),
            })
        } else {
            None
        }
    }

    async fn set(&self, key: SessionKey, session: AtpSession) -> Result<(), SessionStoreError> {
        let key_str = format!("{}_{}", key.0, key.1);
        let stored = StoredPasswordSession {
            access_jwt: session.access_jwt.to_string(),
            refresh_jwt: session.refresh_jwt.to_string(),
            did: session.did.to_string(),
            session_id: key.1.to_string(),
            handle: session.handle.to_string(),
        };
        self.0.set(key_str, stored).await
    }

    async fn del(&self, key: &SessionKey) -> Result<(), SessionStoreError> {
        let key_str = format!("{}_{}", key.0, key.1);
        let entry = Entry::new(&self.0.service, &key_str).map_session_store_err()?;
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuthMethod {
    OAuth,
    AppPassword,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthSession {
    pub did: String,
    pub session_id: String,
    pub store: StoreMethod,
    pub auth: AuthMethod,
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

// There was probably a better way (I hope)
pub enum GenericSession {
    KeyringOAuth(OAuthSession<JacquardResolver, KeyringAuthStore>),
    FileOAuth(OAuthSession<JacquardResolver, FileAuthStore>),
    KeyringPassword(CredentialSession<KeyringAuthStore, JacquardResolver>),
    FilePassword(CredentialSession<FileAuthStore, JacquardResolver>),
}

impl HttpClient for GenericSession {
    type Error = OnyxError;

    async fn send_http(
        &self,
        request: http::Request<Vec<u8>>,
    ) -> core::result::Result<http::Response<Vec<u8>>, Self::Error> {
        match self {
            GenericSession::KeyringOAuth(session) => session
                .send_http(request)
                .await
                .map_err(|e| OnyxError::Auth(e.to_string())),
            GenericSession::FileOAuth(session) => session
                .send_http(request)
                .await
                .map_err(|e| OnyxError::Auth(e.to_string())),
            GenericSession::KeyringPassword(session) => session
                .send_http(request)
                .await
                .map_err(|e| OnyxError::Auth(e.to_string())),
            GenericSession::FilePassword(session) => session
                .send_http(request)
                .await
                .map_err(|e| OnyxError::Auth(e.to_string())),
        }
    }
}

impl XrpcClient for GenericSession {
    async fn base_uri(&self) -> jacquard::CowStr<'static> {
        match self {
            GenericSession::KeyringOAuth(session) => session.base_uri().await,
            GenericSession::FileOAuth(session) => session.base_uri().await,
            GenericSession::KeyringPassword(session) => session.base_uri().await,
            GenericSession::FilePassword(session) => session.base_uri().await,
        }
    }

    async fn opts(&self) -> jacquard::xrpc::CallOptions<'_> {
        match self {
            GenericSession::KeyringOAuth(session) => session.opts().await,
            GenericSession::FileOAuth(session) => session.opts().await,
            GenericSession::KeyringPassword(session) => session.opts().await,
            GenericSession::FilePassword(session) => session.opts().await,
        }
    }

    async fn set_opts(&self, opts: jacquard::xrpc::CallOptions<'_>) {
        match self {
            GenericSession::KeyringOAuth(session) => session.set_opts(opts).await,
            GenericSession::FileOAuth(session) => session.set_opts(opts).await,
            GenericSession::KeyringPassword(session) => session.set_opts(opts).await,
            GenericSession::FilePassword(session) => session.set_opts(opts).await,
        }
    }

    async fn set_base_uri(&self, url: jacquard::url::Url) {
        match self {
            GenericSession::KeyringOAuth(session) => session.set_base_uri(url).await,
            GenericSession::FileOAuth(session) => session.set_base_uri(url).await,
            GenericSession::KeyringPassword(session) => session.set_base_uri(url).await,
            GenericSession::FilePassword(session) => session.set_base_uri(url).await,
        }
    }

    async fn send<R>(&self, request: R) -> XrpcResult<XrpcResponse<R>>
    where
        R: XrpcRequest + Send + Sync,
        <R as XrpcRequest>::Response: Send + Sync,
    {
        match self {
            GenericSession::KeyringOAuth(session) => session.send::<R>(request).await,
            GenericSession::FileOAuth(session) => session.send::<R>(request).await,
            GenericSession::KeyringPassword(session) => session.send::<R>(request).await,
            GenericSession::FilePassword(session) => session.send::<R>(request).await,
        }
    }

    async fn send_with_opts<R>(
        &self,
        request: R,
        opts: jacquard::xrpc::CallOptions<'_>,
    ) -> XrpcResult<XrpcResponse<R>>
    where
        R: XrpcRequest + Send + Sync,
        <R as XrpcRequest>::Response: Send + Sync,
        Self: Sync,
    {
        match self {
            GenericSession::KeyringOAuth(session) => {
                session.send_with_opts::<R>(request, opts).await
            }
            GenericSession::FileOAuth(session) => session.send_with_opts::<R>(request, opts).await,
            GenericSession::KeyringPassword(session) => {
                session.send_with_opts::<R>(request, opts).await
            }
            GenericSession::FilePassword(session) => {
                session.send_with_opts::<R>(request, opts).await
            }
        }
    }
}

impl IdentityResolver for GenericSession {
    fn options(&self) -> &jacquard_identity::resolver::ResolverOptions {
        match self {
            GenericSession::KeyringOAuth(session) => session.options(),
            GenericSession::FileOAuth(session) => session.options(),
            GenericSession::KeyringPassword(session) => session.options(),
            GenericSession::FilePassword(session) => session.options(),
        }
    }

    async fn resolve_handle(
        &self,
        handle: &Handle<'_>,
    ) -> jacquard_identity::resolver::Result<Did<'static>>
    where
        Self: Sync,
    {
        match self {
            GenericSession::KeyringOAuth(session) => session.resolve_handle(handle).await,
            GenericSession::FileOAuth(session) => session.resolve_handle(handle).await,
            GenericSession::KeyringPassword(session) => session.resolve_handle(handle).await,
            GenericSession::FilePassword(session) => session.resolve_handle(handle).await,
        }
    }

    async fn resolve_did_doc(
        &self,
        did: &Did<'_>,
    ) -> jacquard_identity::resolver::Result<jacquard_identity::resolver::DidDocResponse>
    where
        Self: Sync,
    {
        match self {
            GenericSession::KeyringOAuth(session) => session.resolve_did_doc(did).await,
            GenericSession::FileOAuth(session) => session.resolve_did_doc(did).await,
            GenericSession::KeyringPassword(session) => session.resolve_did_doc(did).await,
            GenericSession::FilePassword(session) => session.resolve_did_doc(did).await,
        }
    }
}

impl AgentSession for GenericSession {
    fn session_kind(&self) -> jacquard::client::AgentKind {
        match self {
            GenericSession::KeyringOAuth(_) => jacquard::client::AgentKind::OAuth,
            GenericSession::FileOAuth(_) => jacquard::client::AgentKind::OAuth,
            GenericSession::KeyringPassword(_) => jacquard::client::AgentKind::AppPassword,
            GenericSession::FilePassword(_) => jacquard::client::AgentKind::AppPassword,
        }
    }

    async fn session_info(&self) -> Option<(Did<'static>, Option<jacquard::CowStr<'static>>)> {
        match self {
            GenericSession::KeyringOAuth(session) => {
                let (did, sid) = session.session_info().await;
                Some((did.into_static(), Some(sid.into_static())))
            }
            GenericSession::FileOAuth(session) => {
                let (did, sid) = session.session_info().await;
                Some((did.into_static(), Some(sid.into_static())))
            }
            GenericSession::KeyringPassword(session) => {
                session.session_info().await.map(|key| (key.0, Some(key.1)))
            }
            GenericSession::FilePassword(session) => {
                session.session_info().await.map(|key| (key.0, Some(key.1)))
            }
        }
    }

    async fn endpoint(&self) -> jacquard::CowStr<'static> {
        match self {
            GenericSession::KeyringOAuth(session) => session.endpoint().await,
            GenericSession::FileOAuth(session) => session.endpoint().await,
            GenericSession::KeyringPassword(session) => session.endpoint().await,
            GenericSession::FilePassword(session) => session.endpoint().await,
        }
    }

    async fn set_options<'a>(&'a self, opts: jacquard::xrpc::CallOptions<'a>) {
        match self {
            GenericSession::KeyringOAuth(session) => session.set_options(opts).await,
            GenericSession::FileOAuth(session) => session.set_options(opts).await,
            GenericSession::KeyringPassword(session) => session.set_options(opts).await,
            GenericSession::FilePassword(session) => session.set_options(opts).await,
        }
    }

    async fn refresh(&self) -> jacquard::error::XrpcResult<jacquard::AuthorizationToken<'static>> {
        match self {
            GenericSession::KeyringOAuth(session) => session
                .refresh()
                .await
                .map(|t| t.into_static())
                .map_err(|e| ClientError::transport(e).with_context("OAuth token refresh failed")),

            GenericSession::FileOAuth(session) => session
                .refresh()
                .await
                .map(|t| t.into_static())
                .map_err(|e| ClientError::transport(e).with_context("OAuth token refresh failed")),
            GenericSession::KeyringPassword(session) => session
                .refresh()
                .await
                .map(|t| t.into_static())
                .map_err(|e| {
                    ClientError::transport(e).with_context("App password token refresh failed")
                }),
            GenericSession::FilePassword(session) => session
                .refresh()
                .await
                .map(|t| t.into_static())
                .map_err(|e| {
                    ClientError::transport(e).with_context("App password token refresh failed")
                }),
        }
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
        match password {
            Some(pass) => self.login_app_password(ident, store, pass).await,
            None => self.login_oauth(ident, store).await,
        }
    }

    async fn login_app_password(
        &self,
        ident: &str,
        store_method: StoreMethod,
        password: String,
    ) -> Result<(), OnyxError> {
        let session_id = "session";
        let resolver = PublicResolver::default();

        // TODO: See if there's a clean way to fix this duplication

        if store_method == StoreMethod::Keyring {
            let store = KeyringAuthStore::new(self.service.clone());
            let session = CredentialSession::new(Arc::new(store), Arc::new(resolver));
            let auth = session
                .login(
                    CowStr::Borrowed(ident),
                    CowStr::Borrowed(&password),
                    Some(CowStr::Borrowed(session_id)),
                    None,
                    None,
                    None,
                )
                .await?;
            let auth_session = AuthSession {
                did: auth.did.to_string(),
                session_id: session_id.to_string(),
                store: store_method,
                auth: AuthMethod::AppPassword,
            };
            self.auth_store.set_session(&auth_session)?;
        } else if store_method == StoreMethod::File {
            let store = FileAuthStore::new(self.get_file_store());
            let session = CredentialSession::new(Arc::new(store), Arc::new(resolver));
            let auth = session
                .login(
                    CowStr::Borrowed(ident),
                    CowStr::Borrowed(&password),
                    Some(CowStr::Borrowed(session_id)),
                    None,
                    None,
                    None,
                )
                .await?;
            let auth_session = AuthSession {
                did: auth.did.to_string(),
                session_id: session_id.to_string(),
                store: store_method,
                auth: AuthMethod::AppPassword,
            };
            self.auth_store.set_session(&auth_session)?;
        }

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
                auth: AuthMethod::OAuth,
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
                auth: AuthMethod::OAuth,
            };
            self.auth_store.set_session(&auth_session)?;
        }

        Ok(())
    }

    pub async fn restore(&self) -> Result<GenericSession, OnyxError> {
        let session = match self.auth_store.get_session()? {
            Some(s) => s,
            None => {
                return Err(OnyxError::Auth("not logged in".to_string()));
            }
        };

        match session.auth {
            AuthMethod::OAuth => self.restore_oauth(session).await,
            AuthMethod::AppPassword => self.restore_app_password(session).await,
        }
    }

    async fn restore_app_password(
        &self,
        auth_session: AuthSession,
    ) -> Result<GenericSession, OnyxError> {
        let did = Did::new(&auth_session.did)?;
        let resolver = PublicResolver::default();

        match auth_session.store {
            StoreMethod::Keyring => {
                let store = KeyringAuthStore::new(self.service.clone());
                let session = CredentialSession::new(Arc::new(store), Arc::new(resolver));
                session
                    .restore(did, CowStr::Borrowed(&auth_session.session_id))
                    .await?;
                Ok(GenericSession::KeyringPassword(session))
            }
            StoreMethod::File => {
                let store = FileAuthStore::new(self.get_file_store());
                let session = CredentialSession::new(Arc::new(store), Arc::new(resolver));
                session
                    .restore(did, CowStr::Borrowed(&auth_session.session_id))
                    .await?;
                Ok(GenericSession::FilePassword(session))
            }
        }
    }

    async fn restore_oauth(&self, session: AuthSession) -> Result<GenericSession, OnyxError> {
        let did = Did::new(&session.did)?;

        let client_data = ClientData {
            keyset: None,
            config: AtprotoClientMetadata::default_localhost(),
        };

        match session.store {
            StoreMethod::Keyring => {
                let store = KeyringAuthStore::new(self.service.clone());
                let oauth = OAuthClient::new(store, client_data);
                let session = oauth.restore(&did, &session.session_id).await?;
                Ok(GenericSession::KeyringOAuth(session))
            }
            StoreMethod::File => {
                let store = FileAuthStore::new(self.get_file_store());
                let oauth = OAuthClient::new(store, client_data);
                let session = oauth.restore(&did, &session.session_id).await?;
                Ok(GenericSession::FileOAuth(session))
            }
        }
    }

    pub async fn logout(&self) -> Result<(), OnyxError> {
        let session = match self.auth_store.get_session()? {
            Some(s) => s,
            None => {
                return Err(OnyxError::Auth("not logged in".to_string()));
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

    pub fn get_session_info(&self) -> Result<AuthSession, OnyxError> {
        let session = self.auth_store.get_session()?;
        if let Some(session) = session {
            Ok(session)
        } else {
            Err(OnyxError::Auth("not logged in".to_string()))
        }
    }

    fn get_file_store(&self) -> PathBuf {
        self.config_dir.join("store.json")
    }
}
