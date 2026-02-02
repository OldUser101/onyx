use chrono::{DateTime, FixedOffset};
use jacquard::{CowStr, smol_str::ToSmolStr, types::string::Datetime};

#[derive(Debug)]
pub struct Artist {
    pub artist_name: String,
    pub artist_mb_id: Option<String>,
}

#[derive(Debug)]
pub struct Play {
    pub track_name: String,
    pub track_mb_id: Option<String>,
    pub recording_mb_id: Option<String>,
    pub duration: Option<i64>,
    pub artist_names: Option<Vec<String>>,
    pub artist_mb_ids: Option<Vec<String>>,
    pub artists: Option<Vec<Artist>>,
    pub release_name: Option<String>,
    pub release_mb_id: Option<String>,
    pub isrc: Option<String>,
    pub origin_url: Option<String>,
    pub music_service_base_domain: Option<String>,
    pub submission_client_agent: Option<String>,
    pub played_time: Option<DateTime<FixedOffset>>,
    pub track_discriminant: Option<String>,
    pub release_discriminant: Option<String>,
}

#[derive(Debug)]
pub struct PlayView {
    pub track_name: String,
    pub track_mb_id: Option<String>,
    pub recording_mb_id: Option<String>,
    pub duration: Option<i64>,
    pub artists: Vec<Artist>,
    pub release_name: Option<String>,
    pub release_mb_id: Option<String>,
    pub isrc: Option<String>,
    pub origin_url: Option<String>,
    pub music_service_base_domain: Option<String>,
    pub submission_client_agent: Option<String>,
    pub played_time: Option<DateTime<FixedOffset>>,
}

#[derive(Debug)]
pub struct Status {
    pub time: DateTime<FixedOffset>,
    pub expiry: Option<DateTime<FixedOffset>>,
    pub item: PlayView,
}

impl From<jacquard_api::fm_teal::alpha::feed::Artist<'_>> for Artist {
    fn from(value: jacquard_api::fm_teal::alpha::feed::Artist) -> Self {
        Self {
            artist_name: value.artist_name.to_string(),
            artist_mb_id: value.artist_mb_id.map(|s| s.to_string()),
        }
    }
}

impl From<Artist> for jacquard_api::fm_teal::alpha::feed::Artist<'static> {
    fn from(value: Artist) -> Self {
        Self {
            artist_name: CowStr::Owned(value.artist_name.to_smolstr()),
            artist_mb_id: value.artist_mb_id.map(|s| CowStr::Owned(s.to_smolstr())),
            extra_data: None,
        }
    }
}

impl From<jacquard_api::fm_teal::alpha::feed::play::Play<'_>> for Play {
    fn from(value: jacquard_api::fm_teal::alpha::feed::play::Play<'_>) -> Self {
        Self {
            track_name: value.track_name.to_string(),
            track_mb_id: value.track_mb_id.map(|s| s.to_string()),
            recording_mb_id: value.recording_mb_id.map(|s| s.to_string()),
            duration: value.duration,
            artist_names: value
                .artist_names
                .map(|v| v.iter().map(|a| a.to_string()).collect()),
            artist_mb_ids: value
                .artist_mb_ids
                .map(|v| v.iter().map(|a| a.to_string()).collect()),
            artists: value
                .artists
                .map(|v| v.iter().map(|a| a.clone().into()).collect()),
            release_name: value.release_name.map(|s| s.to_string()),
            release_mb_id: value.release_mb_id.map(|s| s.to_string()),
            isrc: value.isrc.map(|s| s.to_string()),
            origin_url: value.origin_url.map(|s| s.to_string()),
            music_service_base_domain: value.music_service_base_domain.map(|s| s.to_string()),
            submission_client_agent: value.submission_client_agent.map(|s| s.to_string()),
            played_time: value.played_time.map(|dt| *dt.as_ref()),
            track_discriminant: value.track_discriminant.map(|s| s.to_string()),
            release_discriminant: value.release_discriminant.map(|s| s.to_string()),
        }
    }
}

impl From<Play> for jacquard_api::fm_teal::alpha::feed::play::Play<'static> {
    fn from(val: Play) -> Self {
        jacquard_api::fm_teal::alpha::feed::play::Play {
            track_name: CowStr::Owned(val.track_name.to_smolstr()),
            track_mb_id: val.track_mb_id.map(|s| CowStr::Owned(s.to_smolstr())),
            recording_mb_id: val.recording_mb_id.map(|s| CowStr::Owned(s.to_smolstr())),
            duration: val.duration,
            artist_names: val
                .artist_names
                .map(|v| v.iter().map(|s| CowStr::Owned(s.to_smolstr())).collect()),
            artist_mb_ids: val
                .artist_mb_ids
                .map(|v| v.iter().map(|s| CowStr::Owned(s.to_smolstr())).collect()),
            artists: val
                .artists
                .map(|v| v.into_iter().map(|a| a.into()).collect()),
            release_name: val.release_name.map(|s| CowStr::Owned(s.to_smolstr())),
            release_mb_id: val.release_mb_id.map(|s| CowStr::Owned(s.to_smolstr())),
            isrc: val.isrc.map(|s| CowStr::Owned(s.to_smolstr())),
            origin_url: val.origin_url.map(|s| CowStr::Owned(s.to_smolstr())),
            music_service_base_domain: val
                .music_service_base_domain
                .map(|s| CowStr::Owned(s.to_smolstr())),
            submission_client_agent: val
                .submission_client_agent
                .map(|s| CowStr::Owned(s.to_smolstr())),
            played_time: val.played_time.map(Datetime::new),
            track_discriminant: val
                .track_discriminant
                .map(|s| CowStr::Owned(s.to_smolstr())),
            release_discriminant: val
                .release_discriminant
                .map(|s| CowStr::Owned(s.to_smolstr())),
            extra_data: None,
        }
    }
}

impl From<jacquard_api::fm_teal::alpha::feed::PlayView<'_>> for PlayView {
    fn from(value: jacquard_api::fm_teal::alpha::feed::PlayView<'_>) -> Self {
        Self {
            track_name: value.track_name.to_string(),
            track_mb_id: value.track_mb_id.map(|s| s.to_string()),
            recording_mb_id: value.recording_mb_id.map(|s| s.to_string()),
            duration: value.duration,
            artists: value.artists.iter().map(|a| a.clone().into()).collect(),
            release_name: value.release_name.map(|s| s.to_string()),
            release_mb_id: value.release_mb_id.map(|s| s.to_string()),
            isrc: value.isrc.map(|s| s.to_string()),
            origin_url: value.origin_url.map(|s| s.to_string()),
            music_service_base_domain: value.music_service_base_domain.map(|s| s.to_string()),
            submission_client_agent: value.submission_client_agent.map(|s| s.to_string()),
            played_time: value.played_time.map(|dt| *dt.as_ref()),
        }
    }
}

impl From<PlayView> for jacquard_api::fm_teal::alpha::feed::PlayView<'static> {
    fn from(val: PlayView) -> Self {
        jacquard_api::fm_teal::alpha::feed::PlayView {
            track_name: CowStr::Owned(val.track_name.to_smolstr()),
            track_mb_id: val.track_mb_id.map(|s| CowStr::Owned(s.to_smolstr())),
            recording_mb_id: val.recording_mb_id.map(|s| CowStr::Owned(s.to_smolstr())),
            duration: val.duration,
            artists: val.artists.into_iter().map(|a| a.into()).collect(),
            release_name: val.release_name.map(|s| CowStr::Owned(s.to_smolstr())),
            release_mb_id: val.release_mb_id.map(|s| CowStr::Owned(s.to_smolstr())),
            isrc: val.isrc.map(|s| CowStr::Owned(s.to_smolstr())),
            origin_url: val.origin_url.map(|s| CowStr::Owned(s.to_smolstr())),
            music_service_base_domain: val
                .music_service_base_domain
                .map(|s| CowStr::Owned(s.to_smolstr())),
            submission_client_agent: val
                .submission_client_agent
                .map(|s| CowStr::Owned(s.to_smolstr())),
            played_time: val.played_time.map(Datetime::new),
            extra_data: None,
        }
    }
}

impl From<jacquard_api::fm_teal::alpha::actor::status::Status<'_>> for Status {
    fn from(value: jacquard_api::fm_teal::alpha::actor::status::Status<'_>) -> Self {
        Self {
            time: *value.time.as_ref(),
            expiry: value.expiry.map(|dt| *dt.as_ref()),
            item: value.item.into(),
        }
    }
}

impl From<Status> for jacquard_api::fm_teal::alpha::actor::status::Status<'static> {
    fn from(val: Status) -> Self {
        jacquard_api::fm_teal::alpha::actor::status::Status {
            time: Datetime::new(val.time),
            expiry: val.expiry.map(Datetime::new),
            item: val.item.into(),
            extra_data: None,
        }
    }
}
