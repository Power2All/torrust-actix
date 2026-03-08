use crate::tracker::enums::announce_event::AnnounceEvent;
use serde::{
    Deserialize,
    Serialize
};

#[derive(Serialize, Deserialize)]
#[serde(remote = "AnnounceEvent")]
pub enum AnnounceEventDef {
    Started,
    Stopped,
    Completed,
    None,
}