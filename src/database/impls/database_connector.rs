use std::collections::BTreeMap;
use std::sync::Arc;
use sqlx::Error;
use crate::config::structs::configuration::Configuration;
use crate::database::enums::database_drivers::DatabaseDrivers;
use crate::database::structs::database_connector::DatabaseConnector;
use crate::database::structs::database_connector_mysql::DatabaseConnectorMySQL;
use crate::database::structs::database_connector_sqlite::DatabaseConnectorSQLite;
use crate::structs::Cli;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::tracker::structs::user_entry_item::UserEntryItem;
use crate::tracker::structs::user_id::UserId;

impl DatabaseConnector {
    pub async fn new(config: Arc<Configuration>, args: Cli) -> DatabaseConnector
    {
        match &config.database {
            None => { DatabaseConnectorSQLite::database_connector(config, args).await }
            Some(db) => {
                match &db.engine {
                    None => {
                        DatabaseConnectorSQLite::database_connector(config, args).await
                    }
                    Some(db_engine) => {
                        match db_engine {
                            DatabaseDrivers::sqlite3 => { DatabaseConnectorSQLite::database_connector(config, args).await }
                            DatabaseDrivers::mysql => { DatabaseConnectorMySQL::database_connector(config, args).await }
                            // DatabaseDrivers::pgsql => { DatabaseConnectorPgSQL::database_connector(config, args).await }
                        }
                    }
                }
            }
        }
    }

    pub async fn load_torrents(&self, tracker: Arc<TorrentTracker>) -> Result<(u64, u64), Error>
    {
        if self.engine.is_some() {
            return match self.engine.clone().unwrap() {
                DatabaseDrivers::sqlite3 => { self.sqlite.clone().unwrap().load_torrents(tracker.clone()).await }
                DatabaseDrivers::mysql => { self.mysql.clone().unwrap().load_torrents(tracker.clone()).await }
                // DatabaseDrivers::pgsql => { self.pgsql.clone().unwrap().load_torrents(tracker.clone()).await }
            };
        }

        Err(Error::RowNotFound)
    }

    pub async fn load_whitelist(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error>
    {
        if self.engine.is_some() {
            return match self.engine.clone().unwrap() {
                DatabaseDrivers::sqlite3 => { self.sqlite.clone().unwrap().load_whitelist(tracker.clone()).await }
                DatabaseDrivers::mysql => { self.mysql.clone().unwrap().load_whitelist(tracker.clone()).await }
                // DatabaseDrivers::pgsql => { self.pgsql.clone().unwrap().load_whitelist(tracker.clone()).await }
            };
        }

        Err(Error::RowNotFound)
    }

    pub async fn load_blacklist(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error>
    {
        if self.engine.is_some() {
            return match self.engine.clone().unwrap() {
                DatabaseDrivers::sqlite3 => { self.sqlite.clone().unwrap().load_blacklist(tracker.clone()).await }
                DatabaseDrivers::mysql => { self.mysql.clone().unwrap().load_blacklist(tracker.clone()).await }
                // DatabaseDrivers::pgsql => { self.pgsql.clone().unwrap().load_blacklist(tracker.clone()).await }
            };
        }

        Err(Error::RowNotFound)
    }

    pub async fn load_keys(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error>
    {
        if self.engine.is_some() {
            return match self.engine.clone().unwrap() {
                DatabaseDrivers::sqlite3 => { self.sqlite.clone().unwrap().load_keys(tracker.clone()).await }
                DatabaseDrivers::mysql => { self.mysql.clone().unwrap().load_keys(tracker.clone()).await }
                // DatabaseDrivers::pgsql => { self.pgsql.clone().unwrap().load_keys(tracker.clone()).await }
            };
        }

        Err(Error::RowNotFound)
    }

    pub async fn load_users(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error>
    {
        if self.engine.is_some() {
            return match self.engine.clone().unwrap() {
                DatabaseDrivers::sqlite3 => { self.sqlite.clone().unwrap().load_users(tracker.clone()).await }
                DatabaseDrivers::mysql => { self.mysql.clone().unwrap().load_users(tracker.clone()).await }
                // DatabaseDrivers::pgsql => { self.pgsql.clone().unwrap().load_users(tracker.clone()).await }
            };
        }

        Err(Error::RowNotFound)
    }

    pub async fn save_whitelist(&self, tracker: Arc<TorrentTracker>, whitelists: Vec<InfoHash>) -> Result<u64, Error>
    {
        if self.engine.is_some() {
            return match self.engine.clone().unwrap() {
                DatabaseDrivers::sqlite3 => { self.sqlite.clone().unwrap().save_whitelist(tracker.clone(), whitelists).await }
                DatabaseDrivers::mysql => { self.mysql.clone().unwrap().save_whitelist(tracker.clone(), whitelists).await }
                // DatabaseDrivers::pgsql => { self.pgsql.clone().unwrap().save_whitelist(tracker.clone(), whitelists).await }
            };
        }

        Err(Error::RowNotFound)
    }

    pub async fn save_blacklist(&self, tracker: Arc<TorrentTracker>, blacklists: Vec<InfoHash>) -> Result<u64, Error>
    {
        if self.engine.is_some() {
            return match self.engine.clone().unwrap() {
                DatabaseDrivers::sqlite3 => { self.sqlite.clone().unwrap().save_blacklist(tracker.clone(), blacklists).await }
                DatabaseDrivers::mysql => { self.mysql.clone().unwrap().save_blacklist(tracker.clone(), blacklists).await }
                // DatabaseDrivers::pgsql => { self.pgsql.clone().unwrap().save_blacklist(tracker.clone(), blacklists).await }
            };
        }

        Err(Error::RowNotFound)
    }

    pub async fn save_keys(&self, tracker: Arc<TorrentTracker>, keys: BTreeMap<InfoHash, i64>) -> Result<u64, Error>
    {
        if self.engine.is_some() {
            return match self.engine.clone().unwrap() {
                DatabaseDrivers::sqlite3 => { self.sqlite.clone().unwrap().save_keys(tracker.clone(), keys).await }
                DatabaseDrivers::mysql => { self.mysql.clone().unwrap().save_keys(tracker.clone(), keys).await }
                // DatabaseDrivers::pgsql => { self.pgsql.clone().unwrap().save_keys(tracker.clone(), keys).await }
            };
        }

        Err(Error::RowNotFound)
    }

    pub async fn save_torrents(&self, tracker: Arc<TorrentTracker>, torrents: BTreeMap<InfoHash, TorrentEntry>) -> Result<(), Error>
    {
        if self.engine.is_some() {
            return match self.engine.clone().unwrap() {
                DatabaseDrivers::sqlite3 => { self.sqlite.clone().unwrap().save_torrents(tracker.clone(), torrents).await }
                DatabaseDrivers::mysql => { self.mysql.clone().unwrap().save_torrents(tracker.clone(), torrents).await }
                // DatabaseDrivers::pgsql => { self.pgsql.clone().unwrap().save_torrents(tracker.clone(), torrents).await }
            };
        }

        Err(Error::RowNotFound)
    }

    pub async fn save_users(&self, tracker: Arc<TorrentTracker>, users: BTreeMap<UserId, UserEntryItem>) -> Result<(), Error>
    {
        if self.engine.is_some() {
            return match self.engine.clone().unwrap() {
                DatabaseDrivers::sqlite3 => { self.sqlite.clone().unwrap().save_users(tracker.clone(), users).await }
                DatabaseDrivers::mysql => { self.mysql.clone().unwrap().save_users(tracker.clone(), users).await }
                // DatabaseDrivers::pgsql => { self.pgsql.clone().unwrap().save_users(tracker.clone(), users).await }
            };
        }

        Err(Error::RowNotFound)
    }
}
