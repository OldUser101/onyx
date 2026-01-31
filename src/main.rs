use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{
    auth::{AuthMethod, Authenticator, GenericSession},
    error::OnyxError,
    parser::{ParsedArtist, ParsedTrack},
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

        /// The artist name
        #[arg(short, long)]
        artist_name: Option<String>,

        /// The MusicBrainz ID of the artist
        #[arg(long)]
        artist_mb_id: Option<String>,

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

#[derive(Subcommand, Debug)]
enum StatusCommands {
    Show {
        /// Handle or DID to query
        #[arg(long)]
        handle: Option<String>,
    },
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
                        .unwrap_or(&"(no handle)".to_string()))
                    .magenta(),
                    format!(", {}", session_info.did).dimmed()
                );
            }
            AuthCommands::Logout => {
                let auth = get_auth()?;
                let session_info = auth.get_session_info()?;

                auth.logout().await?;

                println!(
                    "{}: logged out {}{}",
                    "success".green().bold(),
                    (session_info
                        .handles
                        .first()
                        .unwrap_or(&"(no handle)".to_string()))
                    .magenta(),
                    format!(", {}", session_info.did).dimmed(),
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
                    println!(
                        "{} {} {} {}",
                        "status:".dimmed(),
                        "logged in".green(),
                        "via".dimmed(),
                        method_str.blue()
                    );
                } else {
                    println!(
                        "{} {} {} {}",
                        "status:".dimmed(),
                        "logged out".red().bold(),
                        "via".dimmed(),
                        method_str.blue()
                    );
                }

                print!("{} ", "handles:".dimmed());

                if session_info.handles.is_empty() {
                    println!("{}", "(no handle)".magenta());
                } else {
                    for handle in &session_info.handles {
                        print!("{} ", handle.magenta());
                    }
                    println!();
                }

                println!("{}", format!("did: {}", session_info.did).dimmed());
            }
        },
        Commands::Scrobble { command } => match command {
            ScrobbleCommands::Track {
                track_name,
                track_mb_id,
                recording_mb_id,
                duration,
                artist_name,
                artist_mb_id,
                release_name,
                release_mb_id,
                origin_url,
                isrc,
                played_time,
                track_discriminant,
                release_discriminant,
            } => {
                let artist = artist_name.map(|a| ParsedArtist {
                    artist_name: a,
                    artist_mb_id,
                });

                let artists = if let Some(artist) = artist {
                    Some(vec![artist])
                } else {
                    None
                };

                let track = ParsedTrack {
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
                    client_id: None,
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
            StatusCommands::Show { handle } => {
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
                status_man.display_status(&status);
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
