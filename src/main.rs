use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{
    auth::{AuthMethod, Authenticator, GenericSession},
    error::OnyxError,
    record::{Artist, Play, PlayView, Status},
    scrobble::Scrobbler,
    status::StatusManager,
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
mod record;
mod scrobble;
mod status;

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
    command: Commands,
}

#[allow(clippy::large_enum_variant)]
#[derive(Subcommand, Debug)]
enum Commands {
    /// Authentication related commands
    Auth {
        #[command(subcommand)]
        command: AuthCommands,
    },

    /// Scrobble tracks
    Scrobble {
        #[command(subcommand)]
        command: ScrobbleCommands,
    },

    /// View and manage listening status
    Status {
        #[command(subcommand)]
        command: StatusCommands,
    },
}

#[derive(Subcommand, Debug)]
enum AuthCommands {
    /// Login with an ATProto handle or DID
    Login {
        /// Handle or DID for login
        handle: String,

        /// Preferred method of storing credentials
        #[arg(short, long, default_value = "keyring")]
        store: StoreMethod,

        /// App password to use, OAuth used if left blank
        #[arg(short, long)]
        password: Option<String>,
    },

    /// Logout of your account
    Logout,

    /// Display logged-in user information
    Whoami,
}

#[derive(Debug, Clone, ValueEnum, Serialize, Deserialize, PartialEq)]
enum StoreMethod {
    /// Use the system keyring, if available
    Keyring,

    /// Save credentials to a file
    File,
}

#[allow(clippy::large_enum_variant)]
#[derive(Subcommand, Debug)]
enum ScrobbleCommands {
    /// Scrobble a single track
    Track {
        /// The name of the track
        track_name: String,

        /// The MusicBrainz ID of the track
        #[arg(long)]
        track_mb_id: Option<String>,

        /// The MusicBrainz ID of the recording
        #[arg(long)]
        recording_mb_id: Option<String>,

        /// The track duration in seconds
        #[arg(short, long)]
        duration: Option<i64>,

        /// A comma-separated list of artist name
        #[arg(short, long)]
        artist_names: Option<String>,

        /// A comma-separated list of artist MusicBrainz IDs
        #[arg(long)]
        artist_mb_ids: Option<String>,

        /// The name of the release/album
        #[arg(short, long)]
        release_name: Option<String>,

        /// The MusicBrainz ID of the release/album
        #[arg(long)]
        release_mb_id: Option<String>,

        /// The URL associated with the track
        #[arg(short, long)]
        origin_url: Option<String>,

        /// The ISRC accosiated with the recording
        #[arg(long)]
        isrc: Option<String>,

        /// Time the track was played (RFC 3339 format)
        #[arg(short, long)]
        played_time: Option<chrono::DateTime<chrono::FixedOffset>>,

        /// Distinguishing information for track variants
        #[arg(long)]
        track_discriminant: Option<String>,

        /// Distinguishing information for release variants
        #[arg(long)]
        release_discriminant: Option<String>,
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

#[allow(clippy::large_enum_variant)]
#[derive(Subcommand, Debug)]
enum StatusCommands {
    /// Display user playing status
    Show {
        /// Handle or DID to query
        #[arg(long)]
        handle: Option<String>,

        /// Display raw status without processing
        #[arg(short, long, action)]
        raw: bool,

        /// Display all status fields
        #[arg(short, long, action)]
        full: bool,
    },

    /// Set user playing status
    Set {
        /// The name of the track
        track_name: String,

        /// The MusicBrainz ID of the track
        #[arg(long)]
        track_mb_id: Option<String>,

        /// The MusicBrainz ID of the recording
        #[arg(long)]
        recording_mb_id: Option<String>,

        /// The track duration in seconds
        #[arg(short, long)]
        duration: Option<i64>,

        /// A comma-separated list of artist name
        #[arg(short, long)]
        artist_names: Option<String>,

        /// A comma-separated list of artist MusicBrainz IDs
        #[arg(long)]
        artist_mb_ids: Option<String>,

        /// The name of the release/album
        #[arg(short, long)]
        release_name: Option<String>,

        /// The MusicBrainz ID of the release/album
        #[arg(long)]
        release_mb_id: Option<String>,

        /// The URL associated with the track
        #[arg(short, long)]
        origin_url: Option<String>,

        /// The ISRC accosiated with the recording
        #[arg(long)]
        isrc: Option<String>,

        /// Time the track was played (RFC 3339 format)
        #[arg(short, long)]
        played_time: Option<chrono::DateTime<chrono::FixedOffset>>,

        /// Time of status creation, defaults to current time
        #[arg(short, long)]
        time: Option<chrono::DateTime<chrono::FixedOffset>>,

        /// Time of status expiry, defaults to start time + 10 minutes
        #[arg(short, long)]
        expiry: Option<chrono::DateTime<chrono::FixedOffset>>,
    },

    /// Clear current playing status
    Clear,
}

fn get_auth() -> Result<Authenticator, OnyxError> {
    let config_dir = dirs::config_dir().unwrap().join("onyx");
    Authenticator::try_new("onyx", &config_dir)
}

async fn get_session() -> Result<GenericSession, OnyxError> {
    let auth = get_auth()?;
    auth.restore().await
}

fn get_command() -> clap::Command {
    Args::command().styles(args_styles())
}

fn generate_client_version() -> String {
    format!("v{}", env!("CARGO_PKG_VERSION"))
}

fn parse_artist_list(
    artist_names: Option<String>,
    artist_mb_ids: Option<String>,
) -> Result<Option<Vec<Artist>>, OnyxError> {
    Ok(match artist_names {
        Some(names) => {
            let mut artists = Vec::new();

            let names: Vec<&str> = names.split(",").collect();
            for name in names {
                let name = name.trim();

                if name.is_empty() {
                    continue;
                }

                artists.push(Artist {
                    artist_name: name.to_owned(),
                    artist_mb_id: None,
                });
            }

            if let Some(mb_ids) = artist_mb_ids {
                let mb_ids: Vec<&str> = mb_ids.split(",").collect();

                if mb_ids.len() > artists.len() {
                    return Err(OnyxError::Parse(
                        "cannot be more `artist_mb_ids` than `artist_names`".into(),
                    ));
                }

                for i in 0..mb_ids.len() {
                    let id = mb_ids[i].trim();

                    if !id.is_empty() {
                        artists[i].artist_mb_id = Some(id.to_owned());
                    }
                }
            }

            Some(artists)
        }
        None => None,
    })
}

async fn run_onyx() -> Result<(), OnyxError> {
    let mut matches = get_command().get_matches();
    let args = Args::from_arg_matches_mut(&mut matches).unwrap();

    match args.command {
        Commands::Auth { command } => match command {
            AuthCommands::Login {
                handle,
                store,
                password,
            } => {
                let auth = get_auth()?;
                auth.login(&handle, store, password).await?;

                let session_info = auth.get_session_info()?;

                println!(
                    "{}: logged in {}{}",
                    "success".green().bold(),
                    (session_info
                        .handles
                        .first()
                        .unwrap_or(&"(no handle)".red().to_string()))
                    .magenta(),
                    format!(", {}", session_info.did).dimmed()
                );
            }
            AuthCommands::Logout => {
                let auth = get_auth()?;
                let session_info = auth.get_session_info()?;

                auth.logout().await?;

                println!(
                    "{}: logged out {}, {}",
                    "success".green().bold(),
                    (session_info
                        .handles
                        .first()
                        .unwrap_or(&"(no handle)".red().to_string())),
                    session_info.did,
                );
            }
            AuthCommands::Whoami => {
                let auth = get_auth()?;
                let session = auth.restore().await;
                let session_info = auth.get_session_info()?;

                let method_str = if session_info.auth == AuthMethod::OAuth {
                    "oauth"
                } else {
                    "app password"
                };

                if session.is_ok() {
                    println!("status: {} via {}", "logged in".green().bold(), method_str);
                } else {
                    println!("status: {} via {}", "logged out".red().bold(), method_str);
                }

                print!("handles: ");

                if session_info.handles.is_empty() {
                    println!("{}", "(no handle)".red());
                } else {
                    for handle in &session_info.handles {
                        print!("{} ", handle);
                    }
                    println!();
                }

                println!("did: {}", session_info.did);
            }
        },
        Commands::Scrobble { command } => match command {
            ScrobbleCommands::Track {
                track_name,
                track_mb_id,
                recording_mb_id,
                duration,
                artist_names,
                artist_mb_ids,
                release_name,
                release_mb_id,
                origin_url,
                isrc,
                played_time,
                track_discriminant,
                release_discriminant,
            } => {
                let artists = parse_artist_list(artist_names, artist_mb_ids)?;

                let track = Play {
                    track_name,
                    track_mb_id,
                    recording_mb_id,
                    duration,
                    artists,
                    release_name,
                    release_mb_id,
                    origin_url,
                    isrc,
                    played_time,
                    track_discriminant,
                    release_discriminant,
                    music_service_base_domain: None,
                    submission_client_agent: None,
                    artist_names: None,
                    artist_mb_ids: None,
                };

                let version = generate_client_version();
                let session = get_session().await?;
                let scrobbler = Scrobbler::new("onyx", &version, session);
                scrobbler.scrobble_track(track).await?;

                println!("{}: track submitted", "success".green().bold());
            }
            ScrobbleCommands::Logfile {
                log,
                log_format,
                delete,
            } => {
                let version = generate_client_version();
                let session = get_session().await?;
                let scrobbler = Scrobbler::new("onyx", &version, session);
                scrobbler.scrobble_logfile(log.clone(), log_format).await?;

                if delete {
                    std::fs::remove_file(&log)?;
                    println!(
                        "{}",
                        format!("deleted log: {}", log.to_str().unwrap()).dimmed()
                    );
                }
            }
        },
        Commands::Status { command } => match command {
            StatusCommands::Show { handle, raw, full } => {
                let ident = match handle {
                    Some(s) => s,
                    None => {
                        let auth = get_auth()?;
                        let session_info = auth.get_session_info()?;
                        session_info.did
                    }
                };

                let status_man = StatusManager::new(&ident);
                let status = status_man.get_status().await?;
                status.display(raw, full);
            }
            StatusCommands::Set {
                track_name,
                track_mb_id,
                recording_mb_id,
                duration,
                artist_names,
                artist_mb_ids,
                release_name,
                release_mb_id,
                origin_url,
                isrc,
                played_time,
                time,
                expiry,
            } => {
                let artists = parse_artist_list(artist_names, artist_mb_ids)?.unwrap_or(Vec::new());

                let play = PlayView {
                    track_name,
                    track_mb_id,
                    recording_mb_id,
                    duration,
                    artists,
                    release_name,
                    release_mb_id,
                    origin_url,
                    isrc,
                    played_time,
                    music_service_base_domain: None,
                    submission_client_agent: None,
                };

                let time = time.unwrap_or(chrono::Local::now().into());

                let status = Status {
                    time,
                    expiry: Some(expiry.unwrap_or(time + std::time::Duration::from_mins(10))),
                    item: play,
                };

                let auth = get_auth()?;
                let session_info = auth.get_session_info()?;
                let session = auth.restore().await?;

                let status_man = StatusManager::new(&session_info.did);
                status_man.set_status(session, status).await?;

                println!(
                    "{}: set status for {}, {}",
                    "success".green().bold(),
                    (session_info
                        .handles
                        .first()
                        .unwrap_or(&"(no handle)".red().to_string())),
                    session_info.did
                );
            }
            StatusCommands::Clear => {
                let auth = get_auth()?;
                let session_info = auth.get_session_info()?;
                let session = auth.restore().await?;

                let status_man = StatusManager::new(&session_info.did);
                status_man.clear_status(session).await?;

                println!(
                    "{}: cleared status for {}, {}",
                    "success".green().bold(),
                    (session_info
                        .handles
                        .first()
                        .unwrap_or(&"(no handle)".red().to_string())),
                    session_info.did,
                );
            }
        },
    }

    Ok(())
}

fn print_error(e: &OnyxError) {
    println!("{}: {}", "error".red().bold(), e);
}

fn handle_error(e: OnyxError) {
    match e {
        OnyxError::Auth(_) => {
            print_error(&e);
            println!(
                "{}: try logging in with '{}'",
                "hint".green().bold(),
                "onyx auth login".cyan().bold()
            );
        }
        _ => print_error(&e),
    }
}

#[tokio::main]
async fn main() {
    if let Err(e) = run_onyx().await {
        handle_error(e);
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_parse_artists() {
        let artist_names = "Test 1 , Test 2 , Test 3, Test 4, ";
        let artist_mb_ids = "ABCD, 1234, DCBA";

        match parse_artist_list(
            Some(artist_names.to_string()),
            Some(artist_mb_ids.to_string()),
        ) {
            Ok(l) => {
                let artists = l.unwrap();

                assert!(artists.len() == 4);

                assert!(artists[0].artist_name == "Test 1");
                assert!(artists[0].artist_mb_id.as_ref().unwrap() == "ABCD");
                assert!(artists[1].artist_name == "Test 2");
                assert!(artists[1].artist_mb_id.as_ref().unwrap() == "1234");
                assert!(artists[2].artist_name == "Test 3");
                assert!(artists[2].artist_mb_id.as_ref().unwrap() == "DCBA");
                assert!(artists[3].artist_name == "Test 4");
                assert!(artists[3].artist_mb_id.is_none());
            }
            Err(e) => {
                panic!("parse_artist_list: {e}");
            }
        }
    }
}
