use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use crate::{
    parser::{LogParser, ParserError},
    record::Play,
};

#[derive(Debug)]
pub struct JsonParser();

impl JsonParser {
    pub fn parse<R>(reader: R) -> Result<Vec<Play>, ParserError>
    where
        R: BufRead,
    {
        let mut plays = Vec::new();

        for play in reader.lines() {
            let play = play?;

            if play.trim().is_empty() {
                // skip over empty lines
                continue;
            }

            let play: Play =
                serde_json::from_str(&play).map_err(|e| ParserError::Syntax(e.to_string()))?;
            plays.push(play);
        }

        Ok(plays)
    }
}

impl LogParser for JsonParser {
    fn parse(log: std::path::PathBuf) -> Result<Vec<Play>, ParserError> {
        let file = File::open(log)?;
        let reader = BufReader::new(file);
        let plays = Self::parse(reader)?;
        Ok(plays)
    }
}
