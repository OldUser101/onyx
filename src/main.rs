use anyhow::Result;
use chrono::{self, DateTime, FixedOffset, Local, TimeZone, Utc};
use jacquard::{
    CowStr,
    client::{Agent, AgentSessionExt, FileAuthStore},
    smol_str::ToSmolStr,
    types::string::Datetime,
};
use jacquard_oauth::{client::OAuthClient, loopback::LoopbackConfig};
use onyx_lexicons::fm_teal::alpha::feed::{Artist, play::Play};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{
    auth::{Authenticator, KeyringAuthStore},
    parser::{ScrobbleLog, ScrobbleRating},
};
use clap::{
    CommandFactory, FromArgMatches, Parser, Subcommand, ValueEnum,
    builder::{
        Styles,
        styling::{AnsiColor, Effects},
    },
};

mod auth;
mod error;
mod parser;

fn args_styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::BrightGreen.on_default().effects(Effects::BOLD))
        .usage(AnsiColor::BrightGreen.on_default().effects(Effects::BOLD))
        .literal(AnsiColor::BrightCyan.on_default().effects(Effects::BOLD))
        .placeholder(AnsiColor::BrightYellow.on_default())
        .valid(AnsiColor::BrightGreen.on_default())
        .invalid(AnsiColor::BrightRed.on_default())
}

#[derive(Parser, Debug)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Authentication related commands
    Auth {
        #[command(subcommand)]
        command: Option<AuthCommands>,
    },

    /// Scrobble tracks
    Scrobble {
        #[command(subcommand)]
        command: Option<ScrobbleCommands>,
    },
}

#[derive(Subcommand, Debug)]
enum AuthCommands {
    /// Login with an ATProto handle or DID
    Login {
        /// Handle to use for login
        ident: String,

        /// Preferred method of storing credentials
        #[arg(short, long, default_value = "keyring")]
        store: StoreMethod,

        /// App password to use, OAuth used if left blank
        #[arg(short, long)]
        password: Option<String>,
    },

    /// Logout of your account
    Logout,

    /// Display user information
    Whoami,
}

#[derive(Debug, Clone, ValueEnum, Serialize, Deserialize, PartialEq)]
enum StoreMethod {
    /// Use the system keyring, if available
    Keyring,

    /// Save credentials to a file
    File,
}

#[derive(Subcommand, Debug)]
enum ScrobbleCommands {
    /// Scrobble a single track
    Track {
        /// Track name
        track_name: CowStr<'static>,
    },

    /// Scrobble tracks from a log file
    Logfile {
        /// Log file path
        log: PathBuf,

        /// Log file format
        log_format: LogFormat,

        /// Delete the log file after processing
        #[arg(short, long, action)]
        delete: bool,
    },
}

#[derive(Debug, Clone, ValueEnum)]
enum LogFormat {
    /// Use AudioScrobbler log format
    AudioScrobbler,
}

#[tokio::main]
async fn main() {
    let mut matches = Args::command().styles(args_styles()).get_matches();
    let args = Args::from_arg_matches_mut(&mut matches).unwrap();

    match args.command {
        Some(Commands::Auth { command }) => match command {
            Some(AuthCommands::Login {
                ident,
                store,
                password,
            }) => {
                let config_dir = dirs::config_dir().unwrap().join("onyx");
                let auth = Authenticator::try_new("onyx", &config_dir).unwrap();

                if let Err(e) = auth.login(&ident, store, password).await {
                    println!("{e}");
                }
            }
            Some(AuthCommands::Logout) => {
                let config_dir = dirs::config_dir().unwrap().join("onyx");
                let auth = Authenticator::try_new("onyx", &config_dir).unwrap();

                if let Err(e) = auth.logout().await {
                    println!("{e}");
                }
            }
            _ => {}
        },
        _ => {}
    }

    /*
    match args.command {
        Some(Commands::Dump { path }) => {
            if let Err(e) = dump_log(path) {
                println!("Error: {e}");
            }
        }
        Some(Commands::Upload {
            handle,
            path,
            store,
        }) => {
            if let Err(e) = upload_log(handle, path, store).await {
                println!("Error: {e}");
            }
        }
        _ => {
            let _ = Args::command().styles(args_styles()).print_long_help();
        }
    }*/
}

fn dump_log(path: PathBuf) -> Result<()> {
    let log = ScrobbleLog::parse_file(path)?;
    println!("{:?}", log);

    Ok(())
}

fn generate_client_agent() -> String {
    format!("onyx/v{}", env!("CARGO_PKG_VERSION"))
}

async fn upload_log(handle: CowStr<'static>, path: PathBuf) -> Result<()> {
    let log = ScrobbleLog::parse_file(path)?;
    let store = KeyringAuthStore::new("onyx".to_string());

    let oauth = OAuthClient::with_default_config(store);

    let session = oauth
        .login_with_local_server(handle, Default::default(), LoopbackConfig::default())
        .await?;

    let agent: Agent<_> = Agent::from(session);

    /*   let client_agent = generate_client_agent();

    for entry in log.entries {
        if entry.rating == ScrobbleRating::Skipped {
            continue;
        }

        let dt: DateTime<FixedOffset> = if let Some(tz) = &log.timezone
            && tz == "UTC"
        {
            Utc.timestamp_opt(entry.timestamp, 0).unwrap().into()
        } else {
            Local.timestamp_opt(entry.timestamp, 0).unwrap().into()
        };

        let mut artists = Vec::new();

        let artist = Artist {
            artist_name: CowStr::Borrowed(&entry.artist_name),
            artist_mb_id: None,
            extra_data: None,
        };

        artists.push(artist);

        let play = Play {
            track_name: CowStr::Borrowed(&entry.track_name),
            duration: Some(entry.duration),
            music_service_base_domain: Some(CowStr::Borrowed("local")),
            played_time: Some(Datetime::new(dt)),
            submission_client_agent: Some(CowStr::Borrowed(&client_agent)),
            artists: Some(artists),
            release_name: entry
                .album_name
                .map(|name| CowStr::Owned(name.to_smolstr())),
            track_mb_id: entry.mb_track_id.map(|id| CowStr::Owned(id.to_smolstr())),
            artist_mb_ids: None,
            artist_names: None,
            isrc: None,
            origin_url: None,
            recording_mb_id: None,
            release_discriminant: None,
            release_mb_id: None,
            track_discriminant: None,
            extra_data: None,
        };

        let _ = agent.create_record(play, None).await?;
        println!("[✓] {}", entry.track_name);
    }*/

    Ok(())
}
