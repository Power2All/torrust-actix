use crate::config::structs::configuration::Configuration;
use crate::database::enums::database_drivers::DatabaseDrivers;
use crate::database::structs::database_connector::DatabaseConnector;
use crate::database::structs::database_connector_mysql::DatabaseConnectorMySQL;
use crate::database::structs::database_connector_pgsql::DatabaseConnectorPgSQL;
use crate::database::structs::database_connector_sqlite::DatabaseConnectorSQLite;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::tracker::structs::user_entry_item::UserEntryItem;
use crate::tracker::structs::user_id::UserId;
use sqlx::Error;
use std::collections::BTreeMap;
use std::sync::Arc;

impl DatabaseConnector {
    pub async fn new(config: Arc<Configuration>, create_database: bool) -> DatabaseConnector
    {
        match &config.database.engine {
            DatabaseDrivers::sqlite3 => { DatabaseConnectorSQLite::database_connector(config, create_database).await }
            DatabaseDrivers::mysql => { DatabaseConnectorMySQL::database_connector(config, create_database).await }
            DatabaseDrivers::pgsql => { DatabaseConnectorPgSQL::database_connector(config, create_database).await }
        }
    }

    pub async fn load_torrents(&self, tracker: Arc<TorrentTracker>) -> Result<(u64, u64), Error>
    {
        let transaction = crate::utils::sentry_tracing::start_trace_transaction("db_load_torrents", "database");
        let result: Result<(u64, u64), Error> = match self.engine.as_ref() {
            Some(DatabaseDrivers::sqlite3) => {
                if let Some(ref sqlite) = self.sqlite {
                    sqlite.load_torrents(tracker).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            Some(DatabaseDrivers::mysql) => {
                if let Some(ref mysql) = self.mysql {
                    mysql.load_torrents(tracker).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            Some(DatabaseDrivers::pgsql) => {
                if let Some(ref pgsql) = self.pgsql {
                    pgsql.load_torrents(tracker).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            None => Err(Error::RowNotFound)
        };
        if let Some(txn) = transaction {
            match &result {
                Ok((loaded, completed)) => {
                    txn.set_tag("result", "success");
                    txn.set_extra("torrents_loaded", (*loaded).into());
                    txn.set_extra("completed_count", (*completed).into());
                }
                Err(e) => {
                    txn.set_tag("result", "error");
                    txn.set_tag("error", e.to_string());
                }
            }
            if let Some(engine) = &self.engine {
                txn.set_tag("database_engine", format!("{:?}", engine));
            }
            txn.finish();
        }
        result
    }

    pub async fn load_whitelist(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error>
    {
        match self.engine.as_ref() {
            Some(DatabaseDrivers::sqlite3) => {
                if let Some(ref sqlite) = self.sqlite {
                    sqlite.load_whitelist(tracker).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            Some(DatabaseDrivers::mysql) => {
                if let Some(ref mysql) = self.mysql {
                    mysql.load_whitelist(tracker).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            Some(DatabaseDrivers::pgsql) => {
                if let Some(ref pgsql) = self.pgsql {
                    pgsql.load_whitelist(tracker).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            None => Err(Error::RowNotFound)
        }
    }

    pub async fn load_blacklist(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error>
    {
        match self.engine.as_ref() {
            Some(DatabaseDrivers::sqlite3) => {
                if let Some(ref sqlite) = self.sqlite {
                    sqlite.load_blacklist(tracker).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            Some(DatabaseDrivers::mysql) => {
                if let Some(ref mysql) = self.mysql {
                    mysql.load_blacklist(tracker).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            Some(DatabaseDrivers::pgsql) => {
                if let Some(ref pgsql) = self.pgsql {
                    pgsql.load_blacklist(tracker).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            None => Err(Error::RowNotFound)
        }
    }

    pub async fn load_keys(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error>
    {
        match self.engine.as_ref() {
            Some(DatabaseDrivers::sqlite3) => {
                if let Some(ref sqlite) = self.sqlite {
                    sqlite.load_keys(tracker).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            Some(DatabaseDrivers::mysql) => {
                if let Some(ref mysql) = self.mysql {
                    mysql.load_keys(tracker).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            Some(DatabaseDrivers::pgsql) => {
                if let Some(ref pgsql) = self.pgsql {
                    pgsql.load_keys(tracker).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            None => Err(Error::RowNotFound)
        }
    }

    pub async fn load_users(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error>
    {
        match self.engine.as_ref() {
            Some(DatabaseDrivers::sqlite3) => {
                if let Some(ref sqlite) = self.sqlite {
                    sqlite.load_users(tracker).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            Some(DatabaseDrivers::mysql) => {
                if let Some(ref mysql) = self.mysql {
                    mysql.load_users(tracker).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            Some(DatabaseDrivers::pgsql) => {
                if let Some(ref pgsql) = self.pgsql {
                    pgsql.load_users(tracker).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            None => Err(Error::RowNotFound)
        }
    }

    pub async fn save_whitelist(&self, tracker: Arc<TorrentTracker>, whitelists: Vec<(InfoHash, UpdatesAction)>) -> Result<u64, Error>
    {
        match self.engine.as_ref() {
            Some(DatabaseDrivers::sqlite3) => {
                if let Some(ref sqlite) = self.sqlite {
                    sqlite.save_whitelist(tracker, whitelists).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            Some(DatabaseDrivers::mysql) => {
                if let Some(ref mysql) = self.mysql {
                    mysql.save_whitelist(tracker, whitelists).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            Some(DatabaseDrivers::pgsql) => {
                if let Some(ref pgsql) = self.pgsql {
                    pgsql.save_whitelist(tracker, whitelists).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            None => Err(Error::RowNotFound)
        }
    }

    pub async fn save_blacklist(&self, tracker: Arc<TorrentTracker>, blacklists: Vec<(InfoHash, UpdatesAction)>) -> Result<u64, Error>
    {
        match self.engine.as_ref() {
            Some(DatabaseDrivers::sqlite3) => {
                if let Some(ref sqlite) = self.sqlite {
                    sqlite.save_blacklist(tracker, blacklists).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            Some(DatabaseDrivers::mysql) => {
                if let Some(ref mysql) = self.mysql {
                    mysql.save_blacklist(tracker, blacklists).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            Some(DatabaseDrivers::pgsql) => {
                if let Some(ref pgsql) = self.pgsql {
                    pgsql.save_blacklist(tracker, blacklists).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            None => Err(Error::RowNotFound)
        }
    }

    pub async fn save_keys(&self, tracker: Arc<TorrentTracker>, keys: BTreeMap<InfoHash, (i64, UpdatesAction)>) -> Result<u64, Error>
    {
        match self.engine.as_ref() {
            Some(DatabaseDrivers::sqlite3) => {
                if let Some(ref sqlite) = self.sqlite {
                    sqlite.save_keys(tracker, keys).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            Some(DatabaseDrivers::mysql) => {
                if let Some(ref mysql) = self.mysql {
                    mysql.save_keys(tracker, keys).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            Some(DatabaseDrivers::pgsql) => {
                if let Some(ref pgsql) = self.pgsql {
                    pgsql.save_keys(tracker, keys).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            None => Err(Error::RowNotFound)
        }
    }

    pub async fn save_torrents(&self, tracker: Arc<TorrentTracker>, torrents: BTreeMap<InfoHash, (TorrentEntry, UpdatesAction)>) -> Result<(), Error>
    {
        let transaction = crate::utils::sentry_tracing::start_trace_transaction("db_save_torrents", "database");
        let result: Result<(), Error> = match self.engine.as_ref() {
            Some(DatabaseDrivers::sqlite3) => {
                if let Some(ref sqlite) = self.sqlite {
                    sqlite.save_torrents(tracker, torrents.clone()).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            Some(DatabaseDrivers::mysql) => {
                if let Some(ref mysql) = self.mysql {
                    mysql.save_torrents(tracker, torrents.clone()).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            Some(DatabaseDrivers::pgsql) => {
                if let Some(ref pgsql) = self.pgsql {
                    pgsql.save_torrents(tracker, torrents.clone()).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            None => Err(Error::RowNotFound)
        };
        if let Some(txn) = transaction {
            match &result {
                Ok(()) => {
                    txn.set_tag("result", "success");
                }
                Err(e) => {
                    txn.set_tag("result", "error");
                    txn.set_tag("error", e.to_string());
                }
            }
            if let Some(engine) = &self.engine {
                txn.set_tag("database_engine", format!("{:?}", engine));
            }
            txn.set_extra("torrents_to_save", (torrents.len() as i64).into());
            txn.finish();
        }
        result
    }

    pub async fn save_users(&self, tracker: Arc<TorrentTracker>, users: BTreeMap<UserId, (UserEntryItem, UpdatesAction)>) -> Result<(), Error>
    {
        match self.engine.as_ref() {
            Some(DatabaseDrivers::sqlite3) => {
                if let Some(ref sqlite) = self.sqlite {
                    sqlite.save_users(tracker, users).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            Some(DatabaseDrivers::mysql) => {
                if let Some(ref mysql) = self.mysql {
                    mysql.save_users(tracker, users).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            Some(DatabaseDrivers::pgsql) => {
                if let Some(ref pgsql) = self.pgsql {
                    pgsql.save_users(tracker, users).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            None => Err(Error::RowNotFound)
        }
    }

    pub async fn reset_seeds_peers(&self, tracker: Arc<TorrentTracker>) -> Result<(), Error>
    {
        match self.engine.as_ref() {
            Some(DatabaseDrivers::sqlite3) => {
                if let Some(ref sqlite) = self.sqlite {
                    sqlite.reset_seeds_peers(tracker).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            Some(DatabaseDrivers::mysql) => {
                if let Some(ref mysql) = self.mysql {
                    mysql.reset_seeds_peers(tracker).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            Some(DatabaseDrivers::pgsql) => {
                if let Some(ref pgsql) = self.pgsql {
                    pgsql.reset_seeds_peers(tracker).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            None => Err(Error::RowNotFound)
        }
    }
}