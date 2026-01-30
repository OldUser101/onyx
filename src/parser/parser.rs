use std::path::PathBuf;

use crate::parser::{ParsedTrack, ParserError};

pub trait LogParser {
    /// Parse the given log file into a list of tracks
    fn parse(log: PathBuf) -> Result<Vec<ParsedTrack>, ParserError>;
}
