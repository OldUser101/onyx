use chrono::{DateTime, FixedOffset};

// See teal.fm lexicons for a description of most of these fields
#[derive(Debug)]
pub struct ParsedArtist {
    pub artist_name: String,
    pub artist_mb_id: Option<String>,
}

// See teal.fm lexicons for a description of most of these fields
#[derive(Debug)]
pub struct ParsedTrack {
    pub track_name: String,
    pub track_mb_id: Option<String>,
    pub recording_mb_id: Option<String>,
    pub duration: Option<i64>,
    pub artist_names: Option<Vec<String>>,
    pub artist_mb_ids: Option<Vec<String>>,
    pub artists: Option<Vec<ParsedArtist>>,
    pub release_name: Option<String>,
    pub release_mb_id: Option<String>,
    pub isrc: Option<String>,
    pub origin_url: Option<String>,
    pub music_service_base_domain: Option<String>,
    pub client_id: Option<String>,
    pub played_time: Option<DateTime<FixedOffset>>,
    pub track_discriminant: Option<String>,
    pub release_discriminant: Option<String>,
}
