use chrono::{DateTime, FixedOffset};
use jacquard::{
    client::{AgentSessionExt, BasicClient},
    prelude::IdentityResolver,
    types::{did::Did, string::Handle},
};
use jacquard_api::fm_teal::alpha::actor::status as fm_teal_status;
use jacquard_identity::{JacquardResolver, PublicResolver};
use owo_colors::OwoColorize;

use crate::error::OnyxError;

#[derive(Debug)]
pub struct ArtistStatus {
    pub artist_name: String,
    pub artist_mb_id: Option<String>,
}

#[derive(Debug)]
pub struct TrackStatus {
    pub time: DateTime<FixedOffset>,
    pub expiry: Option<DateTime<FixedOffset>>,
    pub track_name: String,
    pub track_mb_id: Option<String>,
    pub recording_mb_id: Option<String>,
    pub duration: Option<i64>,
    pub artists: Vec<ArtistStatus>,
    pub release_name: Option<String>,
    pub release_mb_id: Option<String>,
    pub isrc: Option<String>,
    pub origin_url: Option<String>,
    pub music_service_base_domain: Option<String>,
    pub client_id: Option<String>,
    pub played_time: Option<DateTime<FixedOffset>>,
}

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

    pub async fn get_status(&self) -> Result<TrackStatus, OnyxError> {
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

        let artists: Vec<ArtistStatus> = status_rec
            .item
            .artists
            .iter()
            .map(|a| ArtistStatus {
                artist_name: a.artist_name.to_string(),
                artist_mb_id: a.artist_mb_id.clone().map(|s| s.to_string()),
            })
            .collect();

        Ok(TrackStatus {
            time: *status_rec.time.as_ref(),
            expiry: status_rec.expiry.map(|t| *t.as_ref()),
            track_name: status_rec.item.track_name.to_string(),
            track_mb_id: status_rec.item.track_mb_id.map(|s| s.to_string()),
            recording_mb_id: status_rec.item.recording_mb_id.map(|s| s.to_string()),
            duration: status_rec.item.duration,
            artists,
            release_name: status_rec.item.release_name.map(|s| s.to_string()),
            release_mb_id: status_rec.item.release_mb_id.map(|s| s.to_string()),
            isrc: status_rec.item.isrc.map(|s| s.to_string()),
            origin_url: status_rec.item.origin_url.map(|s| s.to_string()),
            music_service_base_domain: status_rec
                .item
                .music_service_base_domain
                .map(|s| s.to_string()),
            client_id: status_rec
                .item
                .submission_client_agent
                .map(|s| s.to_string()),
            played_time: status_rec.item.played_time.map(|t| *t.as_ref()),
        })
    }

    pub fn display_status(&self, status: &TrackStatus, raw: bool, full: bool) {
        // if both track name and artists are blank, probably nothing's playing
        if status.track_name.is_empty() && status.artists.is_empty() && !raw {
            println!("nothing playing right now");
            return;
        }

        println!("track: {}", status.track_name);

        if !status.artists.is_empty() || raw {
            print!("artists: ");

            for i in 0..status.artists.len() {
                print!("{}", status.artists[i].artist_name);

                if i != status.artists.len() - 1 {
                    print!(", ");
                }
            }

            println!();
        }

        if let Some(release) = &status.release_name {
            println!("release: {}", release);
        }

        if let Some(played_time) = &status.played_time {
            if raw {
                println!("played: {}", played_time.format("%Y-%m-%d %H:%M:%S %:z"));
            } else {
                let local_dt = played_time.with_timezone(&chrono::Local);
                println!("played: {}", local_dt.format("%Y-%m-%d %H:%M:%S"));
            }
        }

        if let Some(duration) = status.duration {
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

        if let Some(client) = &status.client_id
            && full
        {
            println!("client: {}", client);
        }
    }
}
