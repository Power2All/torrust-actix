use crate::config::structs::configuration::Configuration;
use crate::database::database::quote_identifier;
use crate::database::enums::database_drivers::DatabaseDrivers;
use crate::database::structs::database_connector::DatabaseConnector;
use crate::database::structs::database_connector_mysql::DatabaseConnectorMySQL;
use crate::database::structs::database_connector_pgsql::DatabaseConnectorPgSQL;
use crate::database::structs::database_connector_sqlite::DatabaseConnectorSQLite;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_update_data::TorrentUpdateData;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::tracker::structs::user_entry_item::UserEntryItem;
use crate::tracker::structs::user_id::UserId;
use log::{
    error,
    warn
};
use sqlx::Error;
use std::collections::BTreeMap;
use std::sync::Arc;

/// True when the failure is about the connection rather than the statement.
///
/// `Error::Io` is what a pooled connection closed by the server surfaces as
/// ("error communicating with database: peer closed connection without sending
/// TLS close_notify"): MySQL/PostgreSQL can drop a connection at any point
/// during a long batch — restart, `wait_timeout`, `KILL`, a proxy in between —
/// and the pool only tests liveness at acquire time, not mid-transaction.
/// Retrying such a failure gets a fresh connection; retrying a rejected
/// statement would just fail identically, so those are not retried.
fn is_transient(e: &Error) -> bool {
    matches!(
        e,
        Error::Io(_) | Error::Tls(_) | Error::Protocol(_) | Error::PoolTimedOut | Error::WorkerCrashed
    )
}

/// Runs a database operation, retrying it once on a dropped connection.
///
/// Also the single place every failure is logged: the engine backends propagate
/// `self.pool.begin()` and `commit_chunk()` failures with a bare `?`, so before
/// this a flush could fail completely silently and the only trace was the
/// caller's "Unable to sync N torrents".
///
/// Engine-agnostic on purpose — MySQL, PostgreSQL and SQLite all route through
/// here, so none of them can take down its sync task with a connection error.
///
/// A macro rather than a function taking an async closure: the resulting future
/// has to stay `Send` for `tokio::spawn`, and an `AsyncFnMut` bound is not
/// higher-ranked enough over the closure's borrows for that to hold. The body is
/// expanded twice, so anything it moves must be cloned per attempt.
macro_rules! with_retry {
    ($what:literal, $body:expr) => {
        match $body {
            Ok(value) => Ok(value),
            Err(e) if is_transient(&e) => {
                warn!("[DATABASE] {}: connection failed ({e}), retrying once on a fresh connection", $what);
                $body.inspect_err(|e| error!("[DATABASE] {}: failed after retry: {e}", $what))
            }
            Err(e) => {
                error!("[DATABASE] {}: failed: {e}", $what);
                Err(e)
            }
        }
    };
}

impl DatabaseConnector {
    /// Connects to the engine selected in the configuration (SQLite 3, MySQL or PostgreSQL),
    /// optionally creating the database schema first.
    pub async fn new(config: Arc<Configuration>, create_database: bool) -> DatabaseConnector
    {
        match &config.database.engine {
            DatabaseDrivers::sqlite3 => { DatabaseConnectorSQLite::database_connector(config, create_database).await }
            DatabaseDrivers::mysql => { DatabaseConnectorMySQL::database_connector(config, create_database).await }
            DatabaseDrivers::pgsql => { DatabaseConnectorPgSQL::database_connector(config, create_database).await }
        }
    }

    /// Loads all persisted torrents into the tracker; returns `(torrents, completed)` counts.
    ///
    /// # Errors
    ///
    /// Returns the underlying `sqlx` error when the database operation fails, or
    /// `Error::RowNotFound` when no backend is initialised for the configured engine.
    pub async fn load_torrents(&self, tracker: Arc<TorrentTracker>) -> Result<(u64, u64), Error>
    {
        let transaction = crate::utils::sentry_tracing::start_trace_transaction("db_load_torrents", "database");
        let result: Result<(u64, u64), Error> = with_retry!("load_torrents", match self.engine.as_ref() {
            Some(DatabaseDrivers::sqlite3) => {
                if let Some(ref sqlite) = self.sqlite {
                    sqlite.load_torrents(tracker.clone()).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            Some(DatabaseDrivers::mysql) => {
                if let Some(ref mysql) = self.mysql {
                    mysql.load_torrents(tracker.clone()).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            Some(DatabaseDrivers::pgsql) => {
                if let Some(ref pgsql) = self.pgsql {
                    pgsql.load_torrents(tracker.clone()).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            None => Err(Error::RowNotFound)
        });
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
                txn.set_tag("database_engine", format!("{engine:?}"));
            }
            txn.finish();
        }
        result
    }

    /// Loads the persisted whitelist into the tracker; returns the number of entries.
    ///
    /// # Errors
    ///
    /// Returns the underlying `sqlx` error when the database operation fails, or
    /// `Error::RowNotFound` when no backend is initialised for the configured engine.
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

    /// Loads the persisted blacklist into the tracker; returns the number of entries.
    ///
    /// # Errors
    ///
    /// Returns the underlying `sqlx` error when the database operation fails, or
    /// `Error::RowNotFound` when no backend is initialised for the configured engine.
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

    /// Loads the persisted announce keys into the tracker; returns the number of entries.
    ///
    /// # Errors
    ///
    /// Returns the underlying `sqlx` error when the database operation fails, or
    /// `Error::RowNotFound` when no backend is initialised for the configured engine.
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

    /// Loads the persisted users into the tracker; returns the number of entries.
    ///
    /// # Errors
    ///
    /// Returns the underlying `sqlx` error when the database operation fails, or
    /// `Error::RowNotFound` when no backend is initialised for the configured engine.
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

    /// Persists whitelist additions/removals; returns the number of rows written.
    ///
    /// # Errors
    ///
    /// Returns the underlying `sqlx` error when the database operation fails, or
    /// `Error::RowNotFound` when no backend is initialised for the configured engine.
    pub async fn save_whitelist(&self, tracker: Arc<TorrentTracker>, whitelists: Vec<(InfoHash, UpdatesAction)>) -> Result<u64, Error>
    {
        with_retry!("save_whitelist", match self.engine.as_ref() {
            Some(DatabaseDrivers::sqlite3) => {
                if let Some(ref sqlite) = self.sqlite {
                    sqlite.save_whitelist(tracker.clone(), whitelists.clone()).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            Some(DatabaseDrivers::mysql) => {
                if let Some(ref mysql) = self.mysql {
                    mysql.save_whitelist(tracker.clone(), whitelists.clone()).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            Some(DatabaseDrivers::pgsql) => {
                if let Some(ref pgsql) = self.pgsql {
                    pgsql.save_whitelist(tracker.clone(), whitelists.clone()).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            None => Err(Error::RowNotFound)
        })
    }

    /// Persists blacklist additions/removals; returns the number of rows written.
    ///
    /// # Errors
    ///
    /// Returns the underlying `sqlx` error when the database operation fails, or
    /// `Error::RowNotFound` when no backend is initialised for the configured engine.
    pub async fn save_blacklist(&self, tracker: Arc<TorrentTracker>, blacklists: Vec<(InfoHash, UpdatesAction)>) -> Result<u64, Error>
    {
        with_retry!("save_blacklist", match self.engine.as_ref() {
            Some(DatabaseDrivers::sqlite3) => {
                if let Some(ref sqlite) = self.sqlite {
                    sqlite.save_blacklist(tracker.clone(), blacklists.clone()).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            Some(DatabaseDrivers::mysql) => {
                if let Some(ref mysql) = self.mysql {
                    mysql.save_blacklist(tracker.clone(), blacklists.clone()).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            Some(DatabaseDrivers::pgsql) => {
                if let Some(ref pgsql) = self.pgsql {
                    pgsql.save_blacklist(tracker.clone(), blacklists.clone()).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            None => Err(Error::RowNotFound)
        })
    }

    /// Persists announce-key additions/removals with their expiry timestamps.
    ///
    /// # Errors
    ///
    /// Returns the underlying `sqlx` error when the database operation fails, or
    /// `Error::RowNotFound` when no backend is initialised for the configured engine.
    pub async fn save_keys(&self, tracker: Arc<TorrentTracker>, keys: BTreeMap<InfoHash, (i64, UpdatesAction)>) -> Result<u64, Error>
    {
        with_retry!("save_keys", match self.engine.as_ref() {
            Some(DatabaseDrivers::sqlite3) => {
                if let Some(ref sqlite) = self.sqlite {
                    sqlite.save_keys(tracker.clone(), keys.clone()).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            Some(DatabaseDrivers::mysql) => {
                if let Some(ref mysql) = self.mysql {
                    mysql.save_keys(tracker.clone(), keys.clone()).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            Some(DatabaseDrivers::pgsql) => {
                if let Some(ref pgsql) = self.pgsql {
                    pgsql.save_keys(tracker.clone(), keys.clone()).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            None => Err(Error::RowNotFound)
        })
    }

    /// Persists a batch of torrent updates, committing in `chunk_size` chunks to keep
    /// transactions (and the locks they hold) short.
    ///
    /// # Errors
    ///
    /// Returns the underlying `sqlx` error when the database operation fails, or
    /// `Error::RowNotFound` when no backend is initialised for the configured engine.
    pub async fn save_torrents(&self, tracker: Arc<TorrentTracker>, torrents: &BTreeMap<InfoHash, (TorrentUpdateData, UpdatesAction)>) -> Result<(), Error>
    {
        let transaction = crate::utils::sentry_tracing::start_trace_transaction("db_save_torrents", "database");
        let result: Result<(), Error> = with_retry!("save_torrents", match self.engine.as_ref() {
            Some(DatabaseDrivers::sqlite3) => {
                if let Some(ref sqlite) = self.sqlite {
                    sqlite.save_torrents(tracker.clone(), torrents).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            Some(DatabaseDrivers::mysql) => {
                if let Some(ref mysql) = self.mysql {
                    mysql.save_torrents(tracker.clone(), torrents).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            Some(DatabaseDrivers::pgsql) => {
                if let Some(ref pgsql) = self.pgsql {
                    pgsql.save_torrents(tracker.clone(), torrents).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            None => Err(Error::RowNotFound)
        });
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
                txn.set_tag("database_engine", format!("{engine:?}"));
            }
            txn.set_extra("torrents_to_save", (torrents.len() as i64).into());
            txn.finish();
        }
        result
    }

    /// Persists a batch of user updates, committing in `chunk_size` chunks to keep
    /// transactions (and the locks they hold) short.
    ///
    /// # Errors
    ///
    /// Returns the underlying `sqlx` error when the database operation fails, or
    /// `Error::RowNotFound` when no backend is initialised for the configured engine.
    pub async fn save_users(&self, tracker: Arc<TorrentTracker>, users: BTreeMap<UserId, (UserEntryItem, UpdatesAction)>) -> Result<(), Error>
    {
        with_retry!("save_users", match self.engine.as_ref() {
            Some(DatabaseDrivers::sqlite3) => {
                if let Some(ref sqlite) = self.sqlite {
                    sqlite.save_users(tracker.clone(), users.clone()).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            Some(DatabaseDrivers::mysql) => {
                if let Some(ref mysql) = self.mysql {
                    mysql.save_users(tracker.clone(), users.clone()).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            Some(DatabaseDrivers::pgsql) => {
                if let Some(ref pgsql) = self.pgsql {
                    pgsql.save_users(tracker.clone(), users.clone()).await
                } else {
                    Err(Error::RowNotFound)
                }
            }
            None => Err(Error::RowNotFound)
        })
    }

    /// Deletes all rows from the given table.
    ///
    /// # Errors
    ///
    /// Returns the underlying `sqlx` error when the database operation fails, or
    /// `Error::RowNotFound` when no backend is initialised for the configured engine.
    pub async fn clear_table(&self, table_name: &str) -> Result<(), Error>
    {
        let query = match self.engine.as_ref() {
            Some(engine) => format!("DELETE FROM {}", quote_identifier(*engine, table_name)),
            None => return Err(Error::RowNotFound),
        };
        match self.engine.as_ref() {
            Some(DatabaseDrivers::sqlite3) => {
                if let Some(ref sqlite) = self.sqlite {
                    sqlx::query(sqlx::AssertSqlSafe(query)).execute(&sqlite.pool).await.map(|_| ())
                } else {
                    Err(Error::RowNotFound)
                }
            }
            Some(DatabaseDrivers::mysql) => {
                if let Some(ref mysql) = self.mysql {
                    sqlx::query(sqlx::AssertSqlSafe(query)).execute(&mysql.pool).await.map(|_| ())
                } else {
                    Err(Error::RowNotFound)
                }
            }
            Some(DatabaseDrivers::pgsql) => {
                if let Some(ref pgsql) = self.pgsql {
                    sqlx::query(sqlx::AssertSqlSafe(query)).execute(&pgsql.pool).await.map(|_| ())
                } else {
                    Err(Error::RowNotFound)
                }
            }
            None => Err(Error::RowNotFound)
        }
    }

    /// Zeroes the seeds and peers columns of every torrent row (used at startup).
    ///
    /// # Errors
    ///
    /// Returns the underlying `sqlx` error when the database operation fails, or
    /// `Error::RowNotFound` when no backend is initialised for the configured engine.
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

#[cfg(test)]
mod tests {
    use super::is_transient;
    use sqlx::Error;

    /// A misclassification here is silent and costly either way: too narrow and a
    /// dropped connection wastes a whole sync cycle, too wide and a permanently
    /// rejected statement gets sent twice on every flush, forever.
    #[test]
    fn dropped_connections_retry_but_rejected_statements_do_not() {
        // Exactly what the reported MySQL failure surfaces as.
        assert!(is_transient(&Error::Io(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "peer closed connection without sending TLS close_notify",
        ))));
        assert!(is_transient(&Error::PoolTimedOut));
        assert!(is_transient(&Error::Protocol("unexpected packet".into())));

        // Retrying these would fail identically, so they must not be retried.
        assert!(!is_transient(&Error::RowNotFound));
        assert!(!is_transient(&Error::PoolClosed));
        assert!(!is_transient(&Error::ColumnNotFound("info_hash".into())));
    }
}