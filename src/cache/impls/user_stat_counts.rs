use crate::cache::structs::user_stat_counts::UserStatCounts;
use crate::tracker::structs::user_entry_item::UserEntryItem;

impl From<&UserEntryItem> for UserStatCounts {
    fn from(user: &UserEntryItem) -> Self {
        Self {
            uploaded: user.uploaded,
            downloaded: user.downloaded,
            completed: user.completed,
            active: user.active,
            updated: user.updated,
            torrents_active: user.torrents_active.len() as u64,
        }
    }
}
