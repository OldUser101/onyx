use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{
    auth::{Authenticator, GenericSession},
    error::OnyxError,
    scrobble::Scrobbler,
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

#[derive(Subcommand, Debug)]
enum ScrobbleCommands {
    /// Scrobble a single track
    Track {
        /// Track name
        track_name: String,
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

#[tokio::main]
async fn main() -> Result<(), OnyxError> {
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
            }
            AuthCommands::Logout => {
                let auth = get_auth()?;
                auth.logout().await?;
            }
            AuthCommands::Whoami => {
                let auth = get_auth()?;
                let session = auth.get_session_info()?;
                println!("logged in as {}", session.did);
            }
        },
        Commands::Scrobble { command } => match command {
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
                    println!("deleted log file {}", log.to_str().unwrap());
                }
            }
            _ => {}
        },
    }

    Ok(())
}
