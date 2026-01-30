use std::path::PathBuf;

use jacquard::client::{Agent, AgentSessionExt};
use jacquard::smol_str::ToSmolStr;
use jacquard::{CowStr, types::string::Datetime};
use jacquard_api::fm_teal::alpha::feed::{Artist, play::Play};
use owo_colors::OwoColorize;

use crate::{
    LogFormat,
    auth::GenericSession,
    error::OnyxError,
    parser::{LogParser, ParsedArtist, ParsedTrack, audio_scrobbler::AudioScrobblerParser},
};

pub struct Scrobbler {
    pub service: String,
    pub version: String,

    agent: Agent<GenericSession>,
}

impl Scrobbler {
    pub fn new(service: &str, version: &str, session: GenericSession) -> Self {
        Self {
            service: service.to_owned(),
            version: version.to_owned(),
            agent: Agent::from(session),
        }
    }

    fn generate_client_agent(&self, id: Option<String>) -> String {
        if let Some(id) = id {
            format!("{}/{} ({})", self.service, self.version, id)
        } else {
            format!("{}/{}", self.service, self.version)
        }
    }

    fn generate_artist(&self, artist: ParsedArtist) -> Artist<'_> {
        Artist {
            artist_name: CowStr::Owned(artist.artist_name.to_smolstr()),
            artist_mb_id: artist.artist_mb_id.map(|s| CowStr::Owned(s.to_smolstr())),
            extra_data: None,
        }
    }

    fn generate_play(&self, track: ParsedTrack) -> Play<'_> {
        let artist_names: Option<Vec<CowStr>> = track.artist_names.map(|v| {
            v.into_iter()
                .map(|s| CowStr::Owned(s.to_smolstr()))
                .collect()
        });

        let artist_mb_ids: Option<Vec<CowStr>> = track.artist_mb_ids.map(|v| {
            v.into_iter()
                .map(|s| CowStr::Owned(s.to_smolstr()))
                .collect()
        });

        let artists: Option<Vec<Artist>> = track
            .artists
            .map(|v| v.into_iter().map(|a| self.generate_artist(a)).collect());

        Play {
            track_name: CowStr::Owned(track.track_name.to_smolstr()),
            track_mb_id: track.track_mb_id.map(|s| CowStr::Owned(s.to_smolstr())),
            recording_mb_id: track.recording_mb_id.map(|s| CowStr::Owned(s.to_smolstr())),
            duration: track.duration,
            artist_names,
            artist_mb_ids,
            artists,
            release_name: track.release_name.map(|s| CowStr::Owned(s.to_smolstr())),
            release_mb_id: track.release_mb_id.map(|s| CowStr::Owned(s.to_smolstr())),
            isrc: track.isrc.map(|s| CowStr::Owned(s.to_smolstr())),
            origin_url: track.origin_url.map(|s| CowStr::Owned(s.to_smolstr())),
            music_service_base_domain: Some(
                track
                    .music_service_base_domain
                    .map(|s| CowStr::Owned(s.to_smolstr()))
                    .unwrap_or(CowStr::Owned("local".to_smolstr())),
            ),
            submission_client_agent: Some(CowStr::Owned(
                self.generate_client_agent(track.client_id).to_smolstr(),
            )),
            played_time: track.played_time.map(Datetime::new),
            track_discriminant: track
                .track_discriminant
                .map(|s| CowStr::Owned(s.to_smolstr())),
            release_discriminant: track
                .release_discriminant
                .map(|s| CowStr::Owned(s.to_smolstr())),
            extra_data: None,
        }
    }

    pub async fn scrobble_track(&self, track: ParsedTrack) -> Result<(), OnyxError> {
        let name = track.track_name.clone();

        let res = async {
            let play = self.generate_play(track);
            self.agent.create_record(play, None).await
        }
        .await;

        if let Err(e) = res {
            println!("{} {}", "[✗]".red().bold(), name);
            return Err(OnyxError::Other(format!("{}, for '{}'", e, name).into()));
        } else {
            println!("{} {}", "[✓]".green().bold(), name);
        }

        Ok(())
    }

    pub async fn scrobble_logfile(
        &self,
        path: PathBuf,
        format: LogFormat,
    ) -> Result<(), OnyxError> {
        println!(
            "{} {}",
            "scrobbling log:".dimmed(),
            path.to_str().unwrap().dimmed()
        );

        let tracks = match format {
            LogFormat::AudioScrobbler => <AudioScrobblerParser as LogParser>::parse(path.clone()),
        }?;

        let count = tracks.len();
        let mut errors = Vec::new();

        for track in tracks {
            if let Err(e) = self.scrobble_track(track).await {
                errors.push(e);
            }
        }

        if !errors.is_empty() {
            println!("\n{}:", "errors".red().bold());

            for error in &errors {
                println!("  - {}", error);
            }

            println!(
                "\n{}: {} tracks submitted, {} failed",
                "summary".yellow().bold(),
                count - errors.len(),
                errors.len()
            );

            return Err(OnyxError::Other(
                format!(
                    "failed to scrobble log file {}, see errors above",
                    path.to_str().unwrap()
                )
                .into(),
            ));
        } else {
            println!("\n{}: {} tracks submitted", "success".green().bold(), count);
        }

        Ok(())
    }
}
