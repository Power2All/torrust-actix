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

/// Forwards each call to the connected engine.
///
/// Every method is the same three-arm match, because [`DatabaseConnector`] is always exactly one
/// engine — there is no "configured but not connected" state left to answer for. Note `with_retry!`
/// expands its body twice, so arguments passed through here must be cloned per attempt.
macro_rules! dispatch {
    ($self:ident, $method:ident $(, $arg:expr)*) => {
        match $self {
            DatabaseConnector::SQLite(backend) => backend.$method($($arg),*).await,
            DatabaseConnector::MySQL(backend) => backend.$method($($arg),*).await,
            DatabaseConnector::PgSQL(backend) => backend.$method($($arg),*).await,
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
        let result: Result<(u64, u64), Error> = with_retry!("load_torrents", dispatch!(self, load_torrents, tracker.clone()));
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
            txn.set_tag("database_engine", format!("{:?}", self.engine()));
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
        dispatch!(self, load_whitelist, tracker)
    }

    /// Loads the persisted blacklist into the tracker; returns the number of entries.
    ///
    /// # Errors
    ///
    /// Returns the underlying `sqlx` error when the database operation fails, or
    /// `Error::RowNotFound` when no backend is initialised for the configured engine.
    pub async fn load_blacklist(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error>
    {
        dispatch!(self, load_blacklist, tracker)
    }

    /// Loads the persisted announce keys into the tracker; returns the number of entries.
    ///
    /// # Errors
    ///
    /// Returns the underlying `sqlx` error when the database operation fails, or
    /// `Error::RowNotFound` when no backend is initialised for the configured engine.
    pub async fn load_keys(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error>
    {
        dispatch!(self, load_keys, tracker)
    }

    /// Loads the persisted users into the tracker; returns the number of entries.
    ///
    /// # Errors
    ///
    /// Returns the underlying `sqlx` error when the database operation fails, or
    /// `Error::RowNotFound` when no backend is initialised for the configured engine.
    pub async fn load_users(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error>
    {
        dispatch!(self, load_users, tracker)
    }

    /// Persists whitelist additions/removals; returns the number of rows written.
    ///
    /// # Errors
    ///
    /// Returns the underlying `sqlx` error when the database operation fails, or
    /// `Error::RowNotFound` when no backend is initialised for the configured engine.
    pub async fn save_whitelist(&self, tracker: Arc<TorrentTracker>, whitelists: Vec<(InfoHash, UpdatesAction)>) -> Result<u64, Error>
    {
        with_retry!("save_whitelist", dispatch!(self, save_whitelist, tracker.clone(), whitelists.clone()))
    }

    /// Persists blacklist additions/removals; returns the number of rows written.
    ///
    /// # Errors
    ///
    /// Returns the underlying `sqlx` error when the database operation fails, or
    /// `Error::RowNotFound` when no backend is initialised for the configured engine.
    pub async fn save_blacklist(&self, tracker: Arc<TorrentTracker>, blacklists: Vec<(InfoHash, UpdatesAction)>) -> Result<u64, Error>
    {
        with_retry!("save_blacklist", dispatch!(self, save_blacklist, tracker.clone(), blacklists.clone()))
    }

    /// Persists announce-key additions/removals with their expiry timestamps.
    ///
    /// # Errors
    ///
    /// Returns the underlying `sqlx` error when the database operation fails, or
    /// `Error::RowNotFound` when no backend is initialised for the configured engine.
    pub async fn save_keys(&self, tracker: Arc<TorrentTracker>, keys: BTreeMap<InfoHash, (i64, UpdatesAction)>) -> Result<u64, Error>
    {
        with_retry!("save_keys", dispatch!(self, save_keys, tracker.clone(), keys.clone()))
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
        let result: Result<(), Error> = with_retry!("save_torrents", dispatch!(self, save_torrents, tracker.clone(), torrents));
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
            txn.set_tag("database_engine", format!("{:?}", self.engine()));
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
        with_retry!("save_users", dispatch!(self, save_users, tracker.clone(), users.clone()))
    }

    /// Deletes all rows from the given table.
    ///
    /// # Errors
    ///
    /// Returns the underlying `sqlx` error when the database operation fails, or
    /// `Error::RowNotFound` when no backend is initialised for the configured engine.
    pub async fn clear_table(&self, table_name: &str) -> Result<(), Error>
    {
        let query = format!("DELETE FROM {}", quote_identifier(self.engine(), table_name));
        // Not the `dispatch!` macro: each pool is a different sqlx type, so `sqlx::query` has to
        // be built inside the arm rather than once outside it.
        match self {
            DatabaseConnector::SQLite(backend) => sqlx::query(sqlx::AssertSqlSafe(query)).execute(&backend.pool).await.map(|_| ()),
            DatabaseConnector::MySQL(backend) => sqlx::query(sqlx::AssertSqlSafe(query)).execute(&backend.pool).await.map(|_| ()),
            DatabaseConnector::PgSQL(backend) => sqlx::query(sqlx::AssertSqlSafe(query)).execute(&backend.pool).await.map(|_| ()),
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
        dispatch!(self, reset_seeds_peers, tracker)
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