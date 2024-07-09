use serde::Deserialize;

#[derive(Deserialize, PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum AnnounceEvent {
    None = 0,
    Completed = 1,
    Started = 2,
    Stopped = 3
}
