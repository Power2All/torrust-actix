/// A user's totals as written to the cache under `<prefix>u:<id>`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UserStatCounts {
    pub uploaded: u64,
    pub downloaded: u64,
    pub completed: u64,
    pub active: u8,
    pub updated: u64,
    /// The number of torrents the user is seeding or leeching.
    pub torrents_active: u64,
}
