use std::path::PathBuf;

use jacquard::client::{Agent, AgentSessionExt};
use jacquard_api::fm_teal::alpha::feed as fm_teal_feed;
use owo_colors::OwoColorize;

use crate::{
    LogFormat,
    auth::GenericSession,
    error::OnyxError,
    parser::{LogParser, audio_scrobbler::AudioScrobblerParser},
    record::Play,
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

    pub async fn scrobble_track(&self, mut track: Play) -> Result<(), OnyxError> {
        let name = track.track_name.clone();

        let res = async {
            track.submission_client_agent =
                Some(self.generate_client_agent(track.submission_client_agent));
            let play: fm_teal_feed::play::Play = track.into();
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
