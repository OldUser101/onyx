use chrono::{DateTime, FixedOffset, Local, TimeZone, Utc};
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
};

use crate::{
    parser::{LogParser, ParserError},
    record::{Artist, Play},
};

#[derive(Debug)]
pub struct AudioScrobblerParser {
    timezone: Option<String>,
    client_id: Option<String>,
    entries: Vec<Scrobble>,
}

#[derive(Debug)]
struct Scrobble {
    artist_name: String,
    album_name: Option<String>,
    track_name: String,
    duration: i64,
    rating: ScrobbleRating,
    timestamp: i64,
    mb_track_id: Option<String>,
}

#[derive(Debug, PartialEq)]
enum ScrobbleRating {
    Listened,
    Skipped,
}

#[derive(Debug)]
enum LogHeaderEntry {
    Version(String),
    TimeZone(String),
    ClientId(String),
    Unknown(()),
}

impl AudioScrobblerParser {
    fn parse_header(line: &str) -> LogHeaderEntry {
        if let Some(rest) = line.strip_prefix("#AUDIOSCROBBLER/") {
            return LogHeaderEntry::Version(rest.to_owned());
        }

        if let Some(rest) = line.strip_prefix("#TZ/") {
            return LogHeaderEntry::TimeZone(rest.to_owned());
        }

        if let Some(rest) = line.strip_prefix("#CLIENT/") {
            return LogHeaderEntry::ClientId(rest.to_owned());
        }

        LogHeaderEntry::Unknown(())
    }

    fn parse_optional_string(s: &str) -> Option<String> {
        if s.is_empty() {
            None
        } else {
            Some(s.to_owned())
        }
    }

    fn parse_rating(s: &str) -> Result<ScrobbleRating, ParserError> {
        if s == "L" {
            Ok(ScrobbleRating::Listened)
        } else if s == "S" {
            Ok(ScrobbleRating::Skipped)
        } else {
            Err(ParserError::Syntax(
                "Entry rating must be 'L' or 'S'".to_string(),
            ))
        }
    }

    fn parse_timezone(s: String) -> Option<String> {
        if s == "UNKNOWN" { None } else { Some(s) }
    }

    fn parse_entry(line: &str, version: &String) -> Result<Scrobble, ParserError> {
        let fields: Vec<&str> = line.split('\t').collect();

        let mb_track_id = if version == "1.1" {
            Self::parse_optional_string(fields[7])
        } else {
            None
        };

        Ok(Scrobble {
            artist_name: fields[0].to_string(),
            album_name: Self::parse_optional_string(fields[1]),
            track_name: fields[2].to_string(),
            duration: fields[4]
                .parse()
                .map_err(|e: std::num::ParseIntError| ParserError::Syntax(e.to_string()))?,
            rating: Self::parse_rating(fields[5])?,
            timestamp: fields[6]
                .parse()
                .map_err(|e: std::num::ParseIntError| ParserError::Syntax(e.to_string()))?,
            mb_track_id,
        })
    }

    pub fn parse<R>(mut reader: R) -> Result<Self, ParserError>
    where
        R: BufRead,
    {
        let mut version: Option<String> = None;
        let mut timezone: Option<String> = None;
        let mut client_id: Option<String> = None;
        let mut entries = Vec::new();

        let mut line = String::new();

        // Parse headers first, since version is needed for entries
        loop {
            line.clear();

            let bytes = reader.read_line(&mut line)?;
            if bytes == 0 {
                break;
            }

            let line = line.trim_end_matches('\n');
            if !line.starts_with('#') {
                break;
            }

            match Self::parse_header(line) {
                LogHeaderEntry::Version(v) => version = Some(v),
                LogHeaderEntry::TimeZone(tz) => timezone = Self::parse_timezone(tz),
                LogHeaderEntry::ClientId(id) => client_id = Some(id),
                _ => {}
            }
        }

        let version =
            version.ok_or_else(|| ParserError::Other("Log version not specified".to_string()))?;

        // Parse entries
        if !line.is_empty() && !line.starts_with('#') {
            let line = line.trim_end_matches('\n');
            entries.push(Self::parse_entry(line, &version)?);
        }

        loop {
            line.clear();

            let bytes = reader.read_line(&mut line)?;
            if bytes == 0 {
                break;
            }

            let line = line.trim_end_matches('\n');
            if line.is_empty() {
                continue;
            }

            entries.push(Self::parse_entry(line, &version)?);
        }

        Ok(Self {
            timezone,
            client_id,
            entries,
        })
    }
}

impl LogParser for AudioScrobblerParser {
    fn parse(log: PathBuf) -> Result<Vec<Play>, ParserError> {
        let file = File::open(log)?;
        let reader = BufReader::new(file);
        let log = Self::parse(reader)?;

        let mut tracks = Vec::new();

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
                artist_name: entry.artist_name,
                artist_mb_id: None,
            };

            artists.push(artist);

            let track = Play {
                track_name: entry.track_name,
                duration: Some(entry.duration),
                played_time: Some(dt),
                submission_client_agent: log.client_id.clone(),
                artists: Some(artists),
                release_name: entry.album_name,
                track_mb_id: entry.mb_track_id,
                music_service_base_domain: None,
                artist_mb_ids: None,
                artist_names: None,
                isrc: None,
                origin_url: None,
                recording_mb_id: None,
                release_mb_id: None,
                track_discriminant: None,
                release_discriminant: None,
            };

            tracks.push(track);
        }

        Ok(tracks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_header_version() {
        let header = AudioScrobblerParser::parse_header("#AUDIOSCROBBLER/1.0");

        if let LogHeaderEntry::Version(v) = header {
            assert_eq!(v, "1.0");
        } else {
            panic!("Expected version header, got {:?}", header);
        }
    }

    #[test]
    fn test_parse_header_time_zone() {
        let header = AudioScrobblerParser::parse_header("#TZ/UTC");

        if let LogHeaderEntry::TimeZone(tz) = header {
            assert_eq!(tz, "UTC");
        } else {
            panic!("Expected time zone header, got {:?}", header);
        }
    }

    #[test]
    fn test_parse_header_client_id() {
        let header = AudioScrobblerParser::parse_header("#CLIENT/Test Client");

        if let LogHeaderEntry::ClientId(id) = header {
            assert_eq!(id, "Test Client");
        } else {
            panic!("Expected client ID header, got {:?}", header);
        }
    }

    #[test]
    fn test_parse_header_unknown() {
        let header = AudioScrobblerParser::parse_header("#SOMETHING ELSE");

        if let LogHeaderEntry::Unknown(s) = header {
            assert_eq!(s, ());
        } else {
            panic!("Expected unknown header, got {:?}", header);
        }
    }

    #[test]
    fn test_parse_entry() {
        let str_log = "#AUDIOSCROBBLER/1.1\nArtist 1\t\tTrack 1\t5\t456\tL\t123456789\tid_0";
        let cur = std::io::Cursor::new(str_log);
        let log = AudioScrobblerParser::parse(cur).unwrap();

        assert_eq!(log.timezone, None);
        assert_eq!(log.client_id, None);

        assert_eq!(log.entries.len(), 1);
        assert_eq!(log.entries[0].artist_name, "Artist 1");
        assert_eq!(log.entries[0].album_name, None);
        assert_eq!(log.entries[0].track_name, "Track 1");
        assert_eq!(log.entries[0].duration, 456);
        assert_eq!(log.entries[0].rating, ScrobbleRating::Listened);
        assert_eq!(log.entries[0].timestamp, 123456789);
        assert_eq!(log.entries[0].mb_track_id, Some("id_0".to_string()));
    }
}
