use crate::tracker::structs::user_entry_item::UserEntryItem;

impl UserEntryItem {
    /// The id the user is cached under: the uuid when it has one, otherwise the numeric id, the
    /// same row identifier the database uses. `None` for an entry carrying neither.
    pub fn cache_id(&self) -> Option<String>
    {
        self.user_uuid.clone().or_else(|| self.user_id.map(|id| id.to_string()))
    }
}
