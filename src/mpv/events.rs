#[derive(Debug, Clone)]
pub enum MpvEvent {
    StartFile,
    EndFile { reason: String },
    TimePos(f32),
    Duration(f32),
    PauseChanged(bool),
    TrackChanged,
}

#[derive(Debug, Clone, Default)]
pub struct MpvState {
    pub is_playing: bool,
    pub is_paused: bool,
    pub current_time: f32,
    pub total_duration: f32,
    pub current_track_index: Option<usize>,
    pub volume: f32,
    pub crashed: bool,
}
