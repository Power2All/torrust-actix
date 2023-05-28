use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserEntryItem {
    pub uuid: String,
    pub key: String,
    pub uploaded: i64,
    pub downloaded: i64,
    pub completed: i64,
    pub updated: i64,
    pub active: i64,
}

impl UserEntryItem {
    pub fn new() -> UserEntryItem {
        UserEntryItem {
            uuid: "".to_string(),
            key: "".to_string(),
            uploaded: 0,
            downloaded: 0,
            completed: 0,
            updated: 0,
            active: 0,
        }
    }
}

impl Default for UserEntryItem {
    fn default() -> Self {
        Self::new()
    }
}
