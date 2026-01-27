use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
};

use anyhow::{Result, anyhow};

#[derive(Debug)]
pub struct ScrobbleLog {
    pub version: String,
    pub timezone: Option<String>,
    pub client_id: Option<String>,
    pub entries: Vec<Scrobble>,
}

#[derive(Debug)]
pub struct Scrobble {
    pub artist_name: String,
    pub album_name: Option<String>,
    pub track_name: String,
    pub track_num: Option<i64>,
    pub duration: i64,
    pub rating: ScrobbleRating,
    pub timestamp: i64,
    pub mb_track_id: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum ScrobbleRating {
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

impl ScrobbleLog {
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

    fn parse_optional_i64(s: &str) -> Option<i64> {
        if s.is_empty() { None } else { s.parse().ok() }
    }

    fn parse_rating(s: &str) -> Result<ScrobbleRating> {
        if s == "L" {
            Ok(ScrobbleRating::Listened)
        } else if s == "S" {
            Ok(ScrobbleRating::Skipped)
        } else {
            Err(anyhow!("Entry rating must be 'L' or 'S'"))
        }
    }

    fn parse_timezone(s: String) -> Option<String> {
        if s == "UNKNOWN" { None } else { Some(s) }
    }

    fn parse_entry(line: &str, version: &String) -> Result<Scrobble> {
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
            track_num: Self::parse_optional_i64(fields[3]),
            duration: fields[4].parse()?,
            rating: Self::parse_rating(fields[5])?,
            timestamp: fields[6].parse()?,
            mb_track_id,
        })
    }

    pub fn parse<R>(mut reader: R) -> Result<Self>
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

        let version = version.ok_or_else(|| anyhow!("Log version not specified"))?;

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
            version,
            timezone,
            client_id,
            entries,
        })
    }

    pub fn parse_file(path: PathBuf) -> Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Self::parse(reader)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_header_version() {
        let header = ScrobbleLog::parse_header("#AUDIOSCROBBLER/1.0");

        if let LogHeaderEntry::Version(v) = header {
            assert_eq!(v, "1.0");
        } else {
            panic!("Expected version header, got {:?}", header);
        }
    }

    #[test]
    fn test_parse_header_time_zone() {
        let header = ScrobbleLog::parse_header("#TZ/UTC");

        if let LogHeaderEntry::TimeZone(tz) = header {
            assert_eq!(tz, "UTC");
        } else {
            panic!("Expected time zone header, got {:?}", header);
        }
    }

    #[test]
    fn test_parse_header_client_id() {
        let header = ScrobbleLog::parse_header("#CLIENT/Test Client");

        if let LogHeaderEntry::ClientId(id) = header {
            assert_eq!(id, "Test Client");
        } else {
            panic!("Expected client ID header, got {:?}", header);
        }
    }

    #[test]
    fn test_parse_header_unknown() {
        let header = ScrobbleLog::parse_header("#SOMETHING ELSE");

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
        let log = ScrobbleLog::parse(cur).unwrap();

        assert_eq!(log.version, "1.1");
        assert_eq!(log.timezone, None);
        assert_eq!(log.client_id, None);

        assert_eq!(log.entries.len(), 1);
        assert_eq!(log.entries[0].artist_name, "Artist 1");
        assert_eq!(log.entries[0].album_name, None);
        assert_eq!(log.entries[0].track_name, "Track 1");
        assert_eq!(log.entries[0].track_num, Some(5));
        assert_eq!(log.entries[0].duration, 456);
        assert_eq!(log.entries[0].rating, ScrobbleRating::Listened);
        assert_eq!(log.entries[0].timestamp, 123456789);
        assert_eq!(log.entries[0].mb_track_id, Some("id_0".to_string()));
    }
}
