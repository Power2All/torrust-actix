use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::tracker::structs::user_entry_item::UserEntryItem;
use crate::tracker::structs::user_id::UserId;
use async_trait::async_trait;
use sqlx::Error;
use std::collections::BTreeMap;
use std::sync::Arc;

#[async_trait]
pub trait DatabaseBackend: Send + Sync {
    async fn load_torrents(&self, tracker: Arc<TorrentTracker>) -> Result<(u64, u64), Error>;

    async fn load_whitelist(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error>;

    async fn load_blacklist(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error>;

    async fn load_keys(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error>;

    async fn load_users(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error>;

    async fn save_torrents(
        &self,
        tracker: Arc<TorrentTracker>,
        torrents: BTreeMap<InfoHash, (TorrentEntry, UpdatesAction)>,
    ) -> Result<(), Error>;

    async fn save_whitelist(
        &self,
        tracker: Arc<TorrentTracker>,
        whitelists: Vec<(InfoHash, UpdatesAction)>,
    ) -> Result<u64, Error>;

    async fn save_blacklist(
        &self,
        tracker: Arc<TorrentTracker>,
        blacklists: Vec<(InfoHash, UpdatesAction)>,
    ) -> Result<u64, Error>;

    async fn save_keys(
        &self,
        tracker: Arc<TorrentTracker>,
        keys: BTreeMap<InfoHash, (i64, UpdatesAction)>,
    ) -> Result<u64, Error>;

    async fn save_users(
        &self,
        tracker: Arc<TorrentTracker>,
        users: BTreeMap<UserId, (UserEntryItem, UpdatesAction)>,
    ) -> Result<(), Error>;

    async fn reset_seeds_peers(&self, tracker: Arc<TorrentTracker>) -> Result<(), Error>;
}