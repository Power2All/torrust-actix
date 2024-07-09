use crate::tracker::enums::announce_event::AnnounceEvent;

impl AnnounceEvent {
    #[inline]
    pub fn from_i32(i: i32) -> Self {
        match i {
            1 => Self::Completed,
            2 => Self::Started,
            3 => Self::Stopped,
            _ => Self::None,
        }
    }

    #[inline]
    pub fn to_i32(&self) -> i32 {
        match self {
            AnnounceEvent::None => 0,
            AnnounceEvent::Completed => 1,
            AnnounceEvent::Started => 2,
            AnnounceEvent::Stopped => 3,
        }
    }
}
