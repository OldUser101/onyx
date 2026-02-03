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

    pub fn display_status(&self, status: &Status, raw: bool, full: bool) {
        // if both track name and artists are blank, probably nothing's playing
        if status.item.track_name.is_empty() && status.item.artists.is_empty() && !raw {
            println!("nothing playing right now");
            return;
        }

        println!("track: {}", status.item.track_name);

        if let Some(track_id) = &status.item.track_mb_id
            && full
        {
            println!("track id: {}", track_id);
        }

        if let Some(recording_id) = &status.item.recording_mb_id
            && full
        {
            println!("recording id: {}", recording_id);
        }

        if !status.item.artists.is_empty() || raw {
            print!("artists: ");

            for i in 0..status.item.artists.len() {
                print!("{}", status.item.artists[i].artist_name);

                if let Some(artist_id) = &status.item.artists[i].artist_mb_id
                    && full
                {
                    print!(" [{}]", artist_id);
                }

                if i != status.item.artists.len() - 1 {
                    print!(", ");
                }
            }

            println!();
        }

        if let Some(release) = &status.item.release_name {
            println!("release: {}", release);
        }

        if let Some(release_id) = &status.item.release_mb_id
            && full
        {
            println!("release id: {}", release_id);
        }

        if let Some(isrc) = &status.item.isrc
            && full
        {
            println!("isrc: {}", isrc);
        }

        if let Some(played_time) = &status.item.played_time {
            if raw {
                println!("played: {}", played_time.format("%Y-%m-%d %H:%M:%S %:z"));
            } else {
                let local_dt = played_time.with_timezone(&chrono::Local);
                println!("played: {}", local_dt.format("%Y-%m-%d %H:%M:%S"));
            }
        }

        if let Some(duration) = status.item.duration {
            if raw {
                println!("duration: {}", duration);
            } else {
                let hours = duration / 3600;
                let minutes = (duration - (hours * 3600)) / 60;
                let seconds = duration - (minutes * 60);

                let mut duration_str = "".to_string();
                if hours > 0 {
                    duration_str = format!("{:02}:", hours);
                }
                if minutes > 0 || hours > 0 {
                    duration_str = format!("{}{:02}:", duration_str, minutes);
                }
                if seconds > 0 || minutes > 0 || hours > 0 {
                    duration_str = format!("{}{:02}", duration_str, seconds);
                }

                println!("duration: {}", duration_str);
            }
        }

        if let Some(service) = &status.item.music_service_base_domain
            && full
        {
            println!("service: {}", service);
        }

        if let Some(client) = &status.item.submission_client_agent
            && full
        {
            println!("client: {}", client);
        }
    }
}
