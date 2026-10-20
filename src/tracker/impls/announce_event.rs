use crate::tracker::enums::announce_event::AnnounceEvent;

impl AnnounceEvent {
    #[inline]
    pub const fn from_i32(i: i32) -> Self {
        match i {
            1 => Self::Completed,
            2 => Self::Started,
            3 => Self::Stopped,
            _ => Self::None,
        }
    }

}