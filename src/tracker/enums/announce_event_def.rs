use serde::{Deserialize, Serialize};
use crate::tracker::enums::announce_event::AnnounceEvent;

#[derive(Serialize, Deserialize)]
#[serde(remote = "AnnounceEvent")]
pub enum AnnounceEventDef {
    Started,
    Stopped,
    Completed,
    None,
}