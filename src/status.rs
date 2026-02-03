use chrono::{DateTime, Duration, FixedOffset};
use jacquard::{
    client::{Agent, AgentSessionExt, BasicClient},
    prelude::IdentityResolver,
    types::{aturi::AtUri, did::Did, string::Handle},
};
use jacquard_api::fm_teal::alpha::actor::status as fm_teal_status;
use jacquard_identity::{JacquardResolver, PublicResolver};

use crate::{
    auth::GenericSession,
    error::OnyxError,
    record::{PlayView, Status},
};

fn get_status_endpoint(did: String) -> String {
    format!("at://{}/fm.teal.alpha.actor.status/self", did)
}

pub struct StatusManager {
    pub ident: String,

    resolver: JacquardResolver,
}

impl StatusManager {
    pub fn new(ident: &str) -> Self {
        Self {
            ident: ident.to_owned(),
            resolver: PublicResolver::default(),
        }
    }

    async fn resolve_did(&self, ident: &str) -> Result<Did<'_>, OnyxError> {
        if let Ok(did) = ident.parse() {
            return Ok(did);
        }

        let handle = Handle::new(ident)?;
        let did = self.resolver.resolve_handle(&handle).await?;
        Ok(did)
    }

    pub async fn get_status(&self) -> Result<Status, OnyxError> {
        let did = self.resolve_did(&self.ident).await?;

        let endpoint = get_status_endpoint(did.to_string());

        let uri = fm_teal_status::Status::uri(&endpoint)?;
        let agent = BasicClient::unauthenticated();

        let response = agent
            .get_record::<fm_teal_status::StatusRecord>(&uri)
            .await?;

        let status_rec = response
            .into_output()
            .map_err(|e| OnyxError::Other(e.to_string().into()))?
            .value;

        Ok(status_rec.into())
    }

    pub async fn set_status(
        &self,
        session: GenericSession,
        status: Status,
    ) -> Result<(), OnyxError> {
        let did = self.resolve_did(&self.ident).await?;
        let endpoint = get_status_endpoint(did.to_string());
        let uri = AtUri::new(&endpoint)?;

        let agent = Agent::from(session);
        agent
            .update_record::<fm_teal_status::Status>(&uri, |stat| {
                let status: fm_teal_status::Status = status.into();
                stat.time = status.time;
                stat.expiry = status.expiry;
                stat.item = status.item;
            })
            .await?;

        Ok(())
    }

    pub async fn clear_status(&self, session: GenericSession) -> Result<(), OnyxError> {
        let now: DateTime<FixedOffset> = chrono::Local::now().into();
        let expiry = now - Duration::minutes(1);

        self.set_status(
            session,
            Status {
                time: now,
                expiry: Some(expiry),
                item: PlayView {
                    track_name: "".to_string(),
                    artists: Vec::new(),
                    ..Default::default()
                },
            },
        )
        .await
    }
}
