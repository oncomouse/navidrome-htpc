#[derive(Debug, Clone, Default)]
pub struct Artist {
    pub id: String,
    pub name: String,
    pub album_count: u32,
    pub cover_art_id: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Album {
    pub id: String,
    pub name: String,
    pub artist_id: String,
    pub artist_name: String,
    pub year: Option<u16>,
    pub genre: Option<String>,
    pub cover_art_id: Option<String>,
    pub song_count: u32,
    pub duration_secs: u32,
    pub created: String,
}

#[derive(Debug, Clone, Default)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub artist_id: String,
    pub artist_name: String,
    pub album_id: String,
    pub album_name: String,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub duration_secs: u32,
    pub cover_art_id: Option<String>,
    pub bitrate: Option<u32>,
    pub suffix: Option<String>,
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub song_count: u32,
    pub duration_secs: u32,
    pub public: Option<bool>,
    pub owner: Option<String>,
    pub created: Option<String>,
}

#[derive(Debug, Clone)]
pub enum QueueSource {
    Album(String),
    Artist(String),
    Playlist(String),
    SearchResult,
    Manual,
}

impl Default for QueueSource {
    fn default() -> Self { Self::Manual }
}
