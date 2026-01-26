use anyhow::Result;
use std::path::PathBuf;

use crate::parser::ScrobbleLog;
use clap::{
    CommandFactory, FromArgMatches, Parser, Subcommand,
    builder::{
        Styles,
        styling::{AnsiColor, Effects},
    },
};

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
    /// Dump a scrobbler log file
    Dump {
        /// File to dump
        #[arg(default_value = ".scrobbler.log")]
        path: PathBuf,
    },
}

fn dump_log(path: PathBuf) -> Result<()> {
    let log = ScrobbleLog::parse_file(path)?;
    println!("{:?}", log);

    Ok(())
}

fn main() {
    let mut matches = Args::command().styles(args_styles()).get_matches();
    let args = Args::from_arg_matches_mut(&mut matches).unwrap();

    match args.command {
        Some(Commands::Dump { path }) => {
            if let Err(e) = dump_log(path) {
                println!("Error: {e}");
            }
        }
        _ => {
            let _ = Args::command().styles(args_styles()).print_long_help();
        }
    }
}
