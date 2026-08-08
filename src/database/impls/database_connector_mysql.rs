use crate::config::structs::configuration::Configuration;
use crate::database::database::{
    build_delete_hash_query,
    build_insert_ignore_hash_query,
    build_select_hash_query,
    build_update_ignore_torrent_query,
    build_upsert_torrent_query,
    limit_offset,
    upsert_conflict_clause
};
use crate::database::enums::database_drivers::DatabaseDrivers;
use crate::database::structs::database_connector::DatabaseConnector;
use crate::database::structs::database_connector_mysql::DatabaseConnectorMySQL;
use crate::database::traits::database_backend::DatabaseBackend;
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_update_data::TorrentUpdateData;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::tracker::structs::user_entry_item::UserEntryItem;
use crate::tracker::structs::user_id::UserId;
use crate::tracker::types::ahash_map::AHashMap;
use async_trait::async_trait;
use futures_util::TryStreamExt;
use log::{
    error,
    info,
    warn
};
use sha1::{
    Digest,
    Sha1
};
use sqlx::mysql::{
    MySqlConnectOptions,
    MySqlPoolOptions
};
use sqlx::{
    ConnectOptions,
    Error,
    MySql,
    Pool,
    Row,
    Transaction
};
use std::collections::BTreeMap;
use std::process::exit;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

const ENGINE: DatabaseDrivers = DatabaseDrivers::mysql;
const LOG_PREFIX: &str = "[MySQL]";

impl DatabaseConnectorMySQL {
    /// Opens a MySQL connection pool from the DSN with statement logging enabled
    /// (schema creation happens in [`Self::database_connector`] when requested).
    ///
    /// # Errors
    ///
    /// Returns the underlying `sqlx` error when the database operation fails.
    pub async fn create(dsl: &str) -> Result<Pool<MySql>, Error> {
        MySqlPoolOptions::new()
            .connect_with(
                MySqlConnectOptions::from_str(dsl)?
                    .log_statements(log::LevelFilter::Debug)
                    .log_slow_statements(log::LevelFilter::Debug, Duration::from_secs(1)),
            )
            .await
    }

    /// Opens the MySQL connection pool from the configured path/DSN, optionally
    /// creating the schema first.
    ///
    /// # Panics / exit
    ///
    /// Exits the process when the connection cannot be established.
    pub async fn database_connector(
        config: Arc<Configuration>,
        create_database: bool,
    ) -> DatabaseConnector {
        let mysql_connect =
            DatabaseConnectorMySQL::create(config.database.clone().path.as_str()).await;
        if let Err(mysql_connect) = mysql_connect {
            error!(
                "{} Unable to connect to MySQL on DSL {}",
                LOG_PREFIX,
                config.database.clone().path
            );
            error!("{LOG_PREFIX} Message: {mysql_connect}");
            exit(1);
        }
        let mut structure = DatabaseConnector {
            mysql: None,
            sqlite: None,
            pgsql: None,
            engine: None,
        };
        structure.mysql = Some(DatabaseConnectorMySQL {
            pool: mysql_connect.unwrap(),
        });
        structure.engine = Some(DatabaseDrivers::mysql);
        if create_database {
            let pool = &structure.mysql.clone().unwrap().pool;
            info!("[BOOT] Database creation triggered for MySQL.");
            let ts = &config.database_structure.torrents;
            let hash_type = if ts.bin_type_infohash { "BINARY(20)" } else { "VARCHAR(40)" };
            info!("[BOOT MySQL] Creating table {}", ts.table_name);
            let query = format!(
                "CREATE TABLE IF NOT EXISTS `{}` (`{}` {} NOT NULL, `{}` INT NOT NULL DEFAULT 0, `{}` INT NOT NULL DEFAULT 0, `{}` BIGINT UNSIGNED NOT NULL DEFAULT 0, PRIMARY KEY (`{}`)) COLLATE='utf8mb4_general_ci'",
                ts.table_name, ts.column_infohash, hash_type, ts.column_seeds, ts.column_peers, ts.column_completed, ts.column_infohash
            );
            if let Err(e) = sqlx::query(sqlx::AssertSqlSafe(query)).execute(pool).await {
                error!("{LOG_PREFIX} Failed to create table {}: {e}", ts.table_name);
                exit(1);
            }
            let ws = &config.database_structure.whitelist;
            let hash_type = if ws.bin_type_infohash { "BINARY(20)" } else { "VARCHAR(40)" };
            info!("[BOOT MySQL] Creating table {}", ws.table_name);
            let query = format!(
                "CREATE TABLE IF NOT EXISTS `{}` (`{}` {} NOT NULL, PRIMARY KEY (`{}`)) COLLATE='utf8mb4_general_ci'",
                ws.table_name, ws.column_infohash, hash_type, ws.column_infohash
            );
            if let Err(e) = sqlx::query(sqlx::AssertSqlSafe(query)).execute(pool).await {
                error!("{LOG_PREFIX} Failed to create table {}: {e}", ws.table_name);
                exit(1);
            }
            let bs = &config.database_structure.blacklist;
            let hash_type = if bs.bin_type_infohash { "BINARY(20)" } else { "VARCHAR(40)" };
            info!("[BOOT MySQL] Creating table {}", bs.table_name);
            let query = format!(
                "CREATE TABLE IF NOT EXISTS `{}` (`{}` {} NOT NULL, PRIMARY KEY (`{}`)) COLLATE='utf8mb4_general_ci'",
                bs.table_name, bs.column_infohash, hash_type, bs.column_infohash
            );
            if let Err(e) = sqlx::query(sqlx::AssertSqlSafe(query)).execute(pool).await {
                error!("{LOG_PREFIX} Failed to create table {}: {e}", bs.table_name);
                exit(1);
            }
            let ks = &config.database_structure.keys;
            let hash_type = if ks.bin_type_hash { "BINARY(20)" } else { "VARCHAR(40)" };
            info!("[BOOT MySQL] Creating table {}", ks.table_name);
            let query = format!(
                "CREATE TABLE IF NOT EXISTS `{}` (`{}` {} NOT NULL, `{}` INT NOT NULL DEFAULT 0, PRIMARY KEY (`{}`)) COLLATE='utf8mb4_general_ci'",
                ks.table_name, ks.column_hash, hash_type, ks.column_timeout, ks.column_hash
            );
            if let Err(e) = sqlx::query(sqlx::AssertSqlSafe(query)).execute(pool).await {
                error!("{LOG_PREFIX} Failed to create table {}: {e}", ks.table_name);
                exit(1);
            }
            let us = &config.database_structure.users;
            let key_type = if us.bin_type_key { "BINARY(20)" } else { "VARCHAR(40)" };
            info!("[BOOT MySQL] Creating table {}", us.table_name);
            let query = if us.id_uuid {
                format!(
                    "CREATE TABLE IF NOT EXISTS `{}` (`{}` VARCHAR(36) NOT NULL, `{}` {} NOT NULL, `{}` BIGINT UNSIGNED NOT NULL DEFAULT 0, `{}` BIGINT UNSIGNED NOT NULL DEFAULT 0, `{}` BIGINT UNSIGNED NOT NULL DEFAULT 0, `{}` TINYINT NOT NULL DEFAULT 0, `{}` INT NOT NULL DEFAULT 0, PRIMARY KEY (`{}`)) COLLATE='utf8mb4_general_ci'",
                    us.table_name, us.column_uuid, us.column_key, key_type, us.column_uploaded, us.column_downloaded, us.column_completed, us.column_active, us.column_updated, us.column_uuid
                )
            } else {
                format!(
                    "CREATE TABLE IF NOT EXISTS `{}` (`{}` BIGINT UNSIGNED NOT NULL AUTO_INCREMENT, `{}` {} NOT NULL, `{}` BIGINT UNSIGNED NOT NULL DEFAULT 0, `{}` BIGINT UNSIGNED NOT NULL DEFAULT 0, `{}` BIGINT UNSIGNED NOT NULL DEFAULT 0, `{}` TINYINT NOT NULL DEFAULT 0, `{}` INT NOT NULL DEFAULT 0, PRIMARY KEY (`{}`)) COLLATE='utf8mb4_general_ci'",
                    us.table_name, us.column_id, us.column_key, key_type, us.column_uploaded, us.column_downloaded, us.column_completed, us.column_active, us.column_updated, us.column_id
                )
            };
            if let Err(e) = sqlx::query(sqlx::AssertSqlSafe(query)).execute(pool).await {
                error!("{LOG_PREFIX} Failed to create table {}: {e}", us.table_name);
                exit(1);
            }
            info!("[BOOT] Created the database and tables, restart without the parameter to start the app.");
        }
        structure
    }

    /// Loads all persisted torrents in pages into the tracker; returns `(torrents, completed)` counts.
    ///
    /// # Errors
    ///
    /// Returns the underlying `sqlx` error when the database operation fails.
    pub async fn load_torrents(&self, tracker: Arc<TorrentTracker>) -> Result<(u64, u64), Error> {
        let mut start = 0u64;
        let length = 100_000_u64;
        let mut torrents = 0u64;
        let mut completed = 0u64;
        let structure = &tracker.config.database_structure.torrents;
        let is_binary = structure.bin_type_infohash;
        loop {
            let query = build_select_hash_query(
                ENGINE,
                &structure.table_name,
                &structure.column_infohash,
                &[&structure.column_completed],
                is_binary,
                start,
                length,
            );
            let mut rows = sqlx::query(sqlx::AssertSqlSafe(query)).fetch(&self.pool);
            while let Some(result) = rows.try_next().await? {
                let info_hash_data: &[u8] = result.get(structure.column_infohash.as_str());
                let info_hash: [u8; 20] =
                    <[u8; 20]>::try_from(&hex::decode(info_hash_data).unwrap()[0..20])
                        .unwrap();
                let completed_count: u64 = result.get::<Option<i64>, _>(structure.column_completed.as_str()).unwrap_or(0) as u64;
                tracker.add_torrent(
                    InfoHash(info_hash),
                    TorrentEntry {
                        seeds: AHashMap::default(),
                        seeds_ipv6: AHashMap::default(),
                        peers: AHashMap::default(),
                        peers_ipv6: AHashMap::default(),
                        rtc_seeds: AHashMap::default(),
                        rtc_peers: AHashMap::default(),
                        completed: completed_count,
                        updated: std::time::Instant::now(),
                    },
                );
                torrents += 1;
                completed += completed_count;
            }
            start += length;
            if torrents < start {
                break;
            }
            info!("{LOG_PREFIX} Handled {torrents} torrents");
        }
        tracker.set_stats(StatsEvent::Completed, completed as i64);
        info!(
            "{LOG_PREFIX} Loaded {torrents} torrents with {completed} completed"
        );
        Ok((torrents, completed))
    }

    /// Persists a batch of torrent updates (insert/update or delete per `UpdatesAction`),
    /// committing every `chunk_size` rows so locks on the torrents table stay short.
    ///
    /// # Errors
    ///
    /// Returns the underlying `sqlx` error when the database operation fails.
    pub async fn save_torrents(
        &self,
        tracker: Arc<TorrentTracker>,
        torrents: &BTreeMap<InfoHash, (TorrentUpdateData, UpdatesAction)>,
    ) -> Result<(), Error> {
        let mut transaction = self.pool.begin().await?;
        let mut handled = 0u64;
        let structure = &tracker.config.database_structure.torrents;
        let db_config = &tracker.config.database;
        let is_binary = structure.bin_type_infohash;
        let chunk_size = db_config.chunk_size;
        let mut in_chunk = 0u64;
        for (info_hash, (counts, updates_action)) in torrents {
            handled += 1;
            let hash_str = info_hash.to_string();
            match updates_action {
                UpdatesAction::Remove => {
                    if db_config.remove_action {
                        let query = build_delete_hash_query(
                            ENGINE,
                            &structure.table_name,
                            &structure.column_infohash,
                            &hash_str,
                            is_binary,
                        );
                        if let Err(e) = sqlx::query(sqlx::AssertSqlSafe(query)).execute(&mut *transaction).await {
                            error!("{LOG_PREFIX} Error: {e}");
                            return Err(e);
                        }
                    }
                }
                UpdatesAction::Add | UpdatesAction::Update => {
                    if db_config.insert_vacant {
                        if db_config.update_peers {
                            let query = build_upsert_torrent_query(
                                ENGINE,
                                &structure.table_name,
                                &structure.column_infohash,
                                &[
                                    (&structure.column_seeds, &counts.seeds_ipv4.to_string()),
                                    (&structure.column_peers, &counts.peers_ipv4.to_string()),
                                ],
                                &[&structure.column_seeds, &structure.column_peers],
                                &hash_str,
                                is_binary,
                            );
                            if let Err(e) = sqlx::query(sqlx::AssertSqlSafe(query)).execute(&mut *transaction).await {
                                error!("{LOG_PREFIX} Error: {e}");
                                return Err(e);
                            }
                        }
                        if db_config.update_completed {
                            let query = build_upsert_torrent_query(
                                ENGINE,
                                &structure.table_name,
                                &structure.column_infohash,
                                &[(&structure.column_completed, &counts.completed.to_string())],
                                &[&structure.column_completed],
                                &hash_str,
                                is_binary,
                            );
                            if let Err(e) = sqlx::query(sqlx::AssertSqlSafe(query)).execute(&mut *transaction).await {
                                error!("{LOG_PREFIX} Error: {e}");
                                return Err(e);
                            }
                        }
                    } else {
                        if db_config.update_peers {
                            let query = build_update_ignore_torrent_query(
                                ENGINE,
                                &structure.table_name,
                                &structure.column_infohash,
                                &[
                                    (&structure.column_seeds, &counts.seeds_ipv4.to_string()),
                                    (&structure.column_peers, &counts.peers_ipv4.to_string()),
                                ],
                                &hash_str,
                                is_binary,
                            );
                            if let Err(e) = sqlx::query(sqlx::AssertSqlSafe(query)).execute(&mut *transaction).await {
                                error!("{LOG_PREFIX} Error: {e}");
                                return Err(e);
                            }
                        }
                        if db_config.update_completed {
                            let query = build_update_ignore_torrent_query(
                                ENGINE,
                                &structure.table_name,
                                &structure.column_infohash,
                                &[(&structure.column_completed, &counts.completed.to_string())],
                                &hash_str,
                                is_binary,
                            );
                            if let Err(e) = sqlx::query(sqlx::AssertSqlSafe(query)).execute(&mut *transaction).await {
                                error!("{LOG_PREFIX} Error: {e}");
                                return Err(e);
                            }
                        }
                    }
                }
            }
            if (handled as f64 / 1000f64).fract() == 0.0 || torrents.len() as u64 == handled {
                info!("{LOG_PREFIX} Handled {handled} torrents");
            }
            transaction = self.commit_chunk(transaction, &mut in_chunk, chunk_size).await?;
        }
        info!("{LOG_PREFIX} Handled {handled} torrents");
        self.commit(transaction).await
    }

    /// Loads the persisted whitelist in pages into the tracker; returns the number of entries.
    ///
    /// # Errors
    ///
    /// Returns the underlying `sqlx` error when the database operation fails.
    pub async fn load_whitelist(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error> {
        let mut start = 0u64;
        let length = 100_000_u64;
        let mut hashes = 0u64;
        let structure = &tracker.config.database_structure.whitelist;
        let is_binary = structure.bin_type_infohash;
        loop {
            let query = build_select_hash_query(
                ENGINE,
                &structure.table_name,
                &structure.column_infohash,
                &[],
                is_binary,
                start,
                length,
            );
            let mut rows = sqlx::query(sqlx::AssertSqlSafe(query)).fetch(&self.pool);
            while let Some(result) = rows.try_next().await? {
                let info_hash_data: &[u8] = result.get(structure.column_infohash.as_str());
                let info_hash: [u8; 20] =
                    <[u8; 20]>::try_from(&hex::decode(info_hash_data).unwrap()[0..20])
                        .unwrap();
                tracker.add_whitelist(InfoHash(info_hash));
                hashes += 1;
            }
            start += length;
            if hashes < start {
                break;
            }
            info!("{LOG_PREFIX} Handled {hashes} whitelisted torrents");
        }
        info!("{LOG_PREFIX} Handled {hashes} whitelisted torrents");
        Ok(hashes)
    }

    /// Persists whitelist additions/removals in a single transaction; returns the rows written.
    ///
    /// # Errors
    ///
    /// Returns the underlying `sqlx` error when the database operation fails.
    pub async fn save_whitelist(
        &self,
        tracker: Arc<TorrentTracker>,
        whitelists: Vec<(InfoHash, UpdatesAction)>,
    ) -> Result<u64, Error> {
        let mut transaction = self.pool.begin().await?;
        let mut handled = 0u64;
        let structure = &tracker.config.database_structure.whitelist;
        let is_binary = structure.bin_type_infohash;
        for (info_hash, updates_action) in &whitelists {
            handled += 1;
            let hash_str = info_hash.to_string();
            match updates_action {
                UpdatesAction::Remove => {
                    if tracker.config.database.remove_action {
                        let query = build_delete_hash_query(
                            ENGINE,
                            &structure.table_name,
                            &structure.column_infohash,
                            &hash_str,
                            is_binary,
                        );
                        if let Err(e) = sqlx::query(sqlx::AssertSqlSafe(query)).execute(&mut *transaction).await {
                            error!("{LOG_PREFIX} Error: {e}");
                            return Err(e);
                        }
                    }
                }
                UpdatesAction::Add | UpdatesAction::Update => {
                    let query = build_insert_ignore_hash_query(
                        ENGINE,
                        &structure.table_name,
                        &structure.column_infohash,
                        &hash_str,
                        is_binary,
                    );
                    if let Err(e) = sqlx::query(sqlx::AssertSqlSafe(query)).execute(&mut *transaction).await {
                        error!("{LOG_PREFIX} Error: {e}");
                        return Err(e);
                    }
                }
            }
            if (handled as f64 / 1000f64).fract() == 0.0 {
                info!("{LOG_PREFIX} Handled {handled} whitelisted torrents");
            }
        }
        info!("{LOG_PREFIX} Handled {handled} whitelisted torrents");
        self.commit(transaction).await?;
        Ok(handled)
    }

    /// Loads the persisted blacklist in pages into the tracker; returns the number of entries.
    ///
    /// # Errors
    ///
    /// Returns the underlying `sqlx` error when the database operation fails.
    pub async fn load_blacklist(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error> {
        let mut start = 0u64;
        let length = 100_000_u64;
        let mut hashes = 0u64;
        let structure = &tracker.config.database_structure.blacklist;
        let is_binary = structure.bin_type_infohash;
        loop {
            let query = build_select_hash_query(
                ENGINE,
                &structure.table_name,
                &structure.column_infohash,
                &[],
                is_binary,
                start,
                length,
            );
            let mut rows = sqlx::query(sqlx::AssertSqlSafe(query)).fetch(&self.pool);
            while let Some(result) = rows.try_next().await? {
                let info_hash_data: &[u8] = result.get(structure.column_infohash.as_str());
                let info_hash: [u8; 20] =
                    <[u8; 20]>::try_from(&hex::decode(info_hash_data).unwrap()[0..20])
                        .unwrap();
                tracker.add_blacklist(InfoHash(info_hash));
                hashes += 1;
            }
            start += length;
            if hashes < start {
                break;
            }
            info!("{LOG_PREFIX} Handled {hashes} blacklisted torrents");
        }
        info!("{LOG_PREFIX} Handled {hashes} blacklisted torrents");
        Ok(hashes)
    }

    /// Persists blacklist additions/removals in a single transaction; returns the rows written.
    ///
    /// # Errors
    ///
    /// Returns the underlying `sqlx` error when the database operation fails.
    pub async fn save_blacklist(
        &self,
        tracker: Arc<TorrentTracker>,
        blacklists: Vec<(InfoHash, UpdatesAction)>,
    ) -> Result<u64, Error> {
        let mut transaction = self.pool.begin().await?;
        let mut handled = 0u64;
        let structure = &tracker.config.database_structure.blacklist;
        let is_binary = structure.bin_type_infohash;
        for (info_hash, updates_action) in &blacklists {
            handled += 1;
            let hash_str = info_hash.to_string();
            match updates_action {
                UpdatesAction::Remove => {
                    if tracker.config.database.remove_action {
                        let query = build_delete_hash_query(
                            ENGINE,
                            &structure.table_name,
                            &structure.column_infohash,
                            &hash_str,
                            is_binary,
                        );
                        if let Err(e) = sqlx::query(sqlx::AssertSqlSafe(query)).execute(&mut *transaction).await {
                            error!("{LOG_PREFIX} Error: {e}");
                            return Err(e);
                        }
                    }
                }
                UpdatesAction::Add | UpdatesAction::Update => {
                    let query = build_insert_ignore_hash_query(
                        ENGINE,
                        &structure.table_name,
                        &structure.column_infohash,
                        &hash_str,
                        is_binary,
                    );
                    if let Err(e) = sqlx::query(sqlx::AssertSqlSafe(query)).execute(&mut *transaction).await {
                        error!("{LOG_PREFIX} Error: {e}");
                        return Err(e);
                    }
                }
            }
            if (handled as f64 / 1000f64).fract() == 0.0 {
                info!("{LOG_PREFIX} Handled {handled} blacklisted torrents");
            }
        }
        info!("{LOG_PREFIX} Handled {handled} blacklisted torrents");
        self.commit(transaction).await?;
        Ok(handled)
    }

    /// Loads the persisted announce keys in pages into the tracker; returns the number of entries.
    ///
    /// # Errors
    ///
    /// Returns the underlying `sqlx` error when the database operation fails.
    pub async fn load_keys(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error> {
        let mut start = 0u64;
        let length = 100_000_u64;
        let mut hashes = 0u64;
        let structure = &tracker.config.database_structure.keys;
        let is_binary = structure.bin_type_hash;
        loop {
            let query = build_select_hash_query(
                ENGINE,
                &structure.table_name,
                &structure.column_hash,
                &[&structure.column_timeout],
                is_binary,
                start,
                length,
            );
            let mut rows = sqlx::query(sqlx::AssertSqlSafe(query)).fetch(&self.pool);
            while let Some(result) = rows.try_next().await? {
                let hash_data: &[u8] = result.get(structure.column_hash.as_str());
                let hash: [u8; 20] =
                    <[u8; 20]>::try_from(&hex::decode(hash_data).unwrap()[0..20]).unwrap();
                let timeout: i64 = result.get(structure.column_timeout.as_str());
                tracker.add_key_absolute(InfoHash(hash), timeout);
                hashes += 1;
            }
            start += length;
            if hashes < start {
                break;
            }
            info!("{LOG_PREFIX} Handled {hashes} keys");
        }
        info!("{LOG_PREFIX} Handled {hashes} keys");
        Ok(hashes)
    }

    /// Persists announce-key additions/removals with expiry timestamps in a single transaction.
    ///
    /// # Errors
    ///
    /// Returns the underlying `sqlx` error when the database operation fails.
    pub async fn save_keys(
        &self,
        tracker: Arc<TorrentTracker>,
        keys: BTreeMap<InfoHash, (i64, UpdatesAction)>,
    ) -> Result<u64, Error> {
        let mut transaction = self.pool.begin().await?;
        let mut handled = 0u64;
        let structure = &tracker.config.database_structure.keys;
        let is_binary = structure.bin_type_hash;
        for (hash, (timeout, update_action)) in &keys {
            handled += 1;
            let hash_str = hash.to_string();
            match update_action {
                UpdatesAction::Remove => {
                    if tracker.config.database.remove_action {
                        let query = build_delete_hash_query(
                            ENGINE,
                            &structure.table_name,
                            &structure.column_hash,
                            &hash_str,
                            is_binary,
                        );
                        if let Err(e) = sqlx::query(sqlx::AssertSqlSafe(query)).execute(&mut *transaction).await {
                            error!("{LOG_PREFIX} Error: {e}");
                            return Err(e);
                        }
                    }
                }
                UpdatesAction::Add | UpdatesAction::Update => {
                    let query = build_upsert_torrent_query(
                        ENGINE,
                        &structure.table_name,
                        &structure.column_hash,
                        &[(&structure.column_timeout, &timeout.to_string())],
                        &[&structure.column_timeout],
                        &hash_str,
                        is_binary,
                    );
                    if let Err(e) = sqlx::query(sqlx::AssertSqlSafe(query)).execute(&mut *transaction).await {
                        error!("{LOG_PREFIX} Error: {e}");
                        return Err(e);
                    }
                }
            }
            if (handled as f64 / 1000f64).fract() == 0.0 {
                info!("{LOG_PREFIX} Handled {handled} keys");
            }
        }
        info!("{LOG_PREFIX} Handled {handled} keys");
        self.commit(transaction).await?;
        Ok(handled)
    }

    /// Loads the persisted users in pages into the tracker; returns the number of entries.
    ///
    /// # Errors
    ///
    /// Returns the underlying `sqlx` error when the database operation fails.
    pub async fn load_users(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error> {
        let mut start = 0u64;
        let length = 100_000_u64;
        let mut hashes = 0u64;
        let structure = &tracker.config.database_structure.users;
        let is_uuid = structure.id_uuid;
        let is_binary_key = structure.bin_type_key;
        loop {
            let id_col = if is_uuid { &structure.column_uuid } else { &structure.column_id };
            let key_select = if is_binary_key {
                format!("HEX(`{}`) AS `{}`", structure.column_key, structure.column_key)
            } else {
                format!("`{}`", structure.column_key)
            };
            let query = format!(
                "SELECT `{}`, {}, `{}`, `{}`, `{}`, `{}`, `{}` FROM `{}` {}",
                id_col,
                key_select,
                structure.column_uploaded,
                structure.column_downloaded,
                structure.column_completed,
                structure.column_updated,
                structure.column_active,
                structure.table_name,
                limit_offset(ENGINE, start, length)
            );
            // Counts rows returned, not users built: a skipped malformed row must still advance
            // the cursor or the loop stops early and silently drops every later page.
            let mut fetched = 0u64;
            let mut rows = sqlx::query(sqlx::AssertSqlSafe(query)).fetch(&self.pool);
            while let Some(result) = rows.try_next().await? {
                fetched += 1;
                let hash = if is_uuid {
                    let uuid_data: &[u8] = result.get(structure.column_uuid.as_str());
                    let mut hasher = Sha1::new();
                    hasher.update(uuid_data);
                    <[u8; 20]>::try_from(hasher.finalize().as_slice()).unwrap()
                } else {
                    let id_data: &[u8] = result.get(structure.column_id.as_str());
                    let mut hasher = Sha1::new();
                    hasher.update(id_data);
                    <[u8; 20]>::try_from(hasher.finalize().as_slice()).unwrap()
                };
                // A malformed key column would otherwise abort the process at boot; skip the row
                // and let the operator see which user is broken.
                let Ok(user_key) = UserId::from_str(result.get(structure.column_key.as_str())) else {
                    warn!("{LOG_PREFIX} Skipping user with an unparsable key column");
                    continue;
                };
                tracker.add_user(
                    UserId(hash),
                    UserEntryItem {
                        key: user_key,
                        user_id: if is_uuid {
                            None
                        } else {
                            Some(result.get(structure.column_id.as_str()))
                        },
                        user_uuid: if is_uuid {
                            Some(result.get(structure.column_uuid.as_str()))
                        } else {
                            None
                        },
                        uploaded: result.get::<i64, &str>(structure.column_uploaded.as_str()) as u64,
                        downloaded: result.get::<i64, &str>(structure.column_downloaded.as_str())
                            as u64,
                        completed: result.get::<i64, &str>(structure.column_completed.as_str())
                            as u64,
                        updated: result.get::<i32, &str>(structure.column_updated.as_str()) as u64,
                        active: result.get::<i8, &str>(structure.column_active.as_str()) as u8,
                        torrents_active: Default::default(),
                    },
                );
                hashes += 1;
            }
            if fetched < length {
                break;
            }
            start += length;
            info!("{LOG_PREFIX} Loaded {hashes} users");
        }
        info!("{LOG_PREFIX} Loaded {hashes} users");
        Ok(hashes)
    }

    /// Persists a batch of user updates (upsert or delete per `UpdatesAction`),
    /// committing every `chunk_size` rows so transactions stay short.
    ///
    /// # Errors
    ///
    /// Returns the underlying `sqlx` error when the database operation fails.
    pub async fn save_users(
        &self,
        tracker: Arc<TorrentTracker>,
        users: BTreeMap<UserId, (UserEntryItem, UpdatesAction)>,
    ) -> Result<(), Error> {
        let mut transaction = self.pool.begin().await?;
        let mut handled = 0u64;
        let structure = &tracker.config.database_structure.users;
        let db_config = &tracker.config.database;
        let is_uuid = structure.id_uuid;
        let is_binary_key = structure.bin_type_key;
        let chunk_size = db_config.chunk_size;
        let mut in_chunk = 0u64;
        for (user_entry_item, updates_action) in users.values() {
            handled += 1;
            // Resolve the row identifier once. Rows are addressed by UUID or by numeric id
            // depending on `id_uuid`, and an entry carrying neither (or the other one, after
            // an `id_uuid` flip) gives every branch below nothing to target. Unwrapping it
            // would take the whole process down, since the release profile sets
            // `panic = 'abort'`.
            let (id_col, id_val, id_uuid_raw) = if is_uuid {
                match user_entry_item.user_uuid.as_ref() {
                    Some(uuid) => (&structure.column_uuid, format!("'{uuid}'"), Some(uuid.clone())),
                    None => {
                        warn!("{LOG_PREFIX} Skipping user update with no uuid set");
                        continue;
                    }
                }
            } else {
                match user_entry_item.user_id {
                    Some(user_id) => (&structure.column_id, user_id.to_string(), None),
                    None => {
                        warn!("{LOG_PREFIX} Skipping user update with no id set");
                        continue;
                    }
                }
            };
            match updates_action {
                UpdatesAction::Remove => {
                    if db_config.remove_action {
                        // The UUID is the only identifier that is a string, so it is bound
                        // rather than interpolated: a `DELETE` is the statement least worth
                        // trusting to upstream validation. The numeric id is a `u64` and
                        // cannot render as anything but digits.
                        let query = format!(
                            "DELETE FROM `{}` WHERE `{}`={}",
                            structure.table_name,
                            id_col,
                            if id_uuid_raw.is_some() { "?" } else { id_val.as_str() }
                        );
                        let mut statement = sqlx::query(sqlx::AssertSqlSafe(query));
                        if let Some(uuid) = id_uuid_raw {
                            statement = statement.bind(uuid);
                        }
                        if let Err(e) = statement.execute(&mut *transaction).await {
                            error!("{LOG_PREFIX} Error: {e}");
                            return Err(e);
                        }
                    }
                }
                UpdatesAction::Add | UpdatesAction::Update => {
                    let key_value = if is_binary_key {
                        format!("UNHEX('{}')", user_entry_item.key)
                    } else {
                        format!("'{}'", user_entry_item.key)
                    };
                    let query = if db_config.insert_vacant {
                        let conflict_clause = upsert_conflict_clause(
                            ENGINE,
                            id_col,
                            &[
                                &structure.column_key,
                                &structure.column_uploaded,
                                &structure.column_downloaded,
                                &structure.column_completed,
                                &structure.column_active,
                                &structure.column_updated,
                            ],
                        );
                        format!(
                            "INSERT INTO `{}` (`{}`, `{}`, `{}`, `{}`, `{}`, `{}`, `{}`) VALUES ({}, {}, {}, {}, {}, {}, {}) {}",
                            structure.table_name,
                            id_col,
                            structure.column_key,
                            structure.column_uploaded,
                            structure.column_downloaded,
                            structure.column_completed,
                            structure.column_active,
                            structure.column_updated,
                            id_val,
                            key_value,
                            user_entry_item.uploaded,
                            user_entry_item.downloaded,
                            user_entry_item.completed,
                            user_entry_item.active,
                            user_entry_item.updated,
                            conflict_clause
                        )
                    } else {
                        format!(
                            "UPDATE IGNORE `{}` SET `{}`={}, `{}`={}, `{}`={}, `{}`={}, `{}`={}, `{}`={} WHERE `{}`={}",
                            structure.table_name,
                            structure.column_key,
                            key_value,
                            structure.column_uploaded,
                            user_entry_item.uploaded,
                            structure.column_downloaded,
                            user_entry_item.downloaded,
                            structure.column_completed,
                            user_entry_item.completed,
                            structure.column_active,
                            user_entry_item.active,
                            structure.column_updated,
                            user_entry_item.updated,
                            id_col,
                            id_val
                        )
                    };
                    if let Err(e) = sqlx::query(sqlx::AssertSqlSafe(query)).execute(&mut *transaction).await {
                        error!("{LOG_PREFIX} Error: {e}");
                        return Err(e);
                    }
                }
            }
            if (handled as f64 / 1000f64).fract() == 0.0 || users.len() as u64 == handled {
                info!("{LOG_PREFIX} Handled {handled} users");
            }
            transaction = self.commit_chunk(transaction, &mut in_chunk, chunk_size).await?;
        }
        info!("{LOG_PREFIX} Handled {handled} users");
        self.commit(transaction).await
    }

    /// Zeroes the seeds and peers columns of every torrent row.
    ///
    /// # Errors
    ///
    /// Returns the underlying `sqlx` error when the database operation fails.
    pub async fn reset_seeds_peers(&self, tracker: Arc<TorrentTracker>) -> Result<(), Error> {
        let mut transaction = self.pool.begin().await?;
        let structure = &tracker.config.database_structure.torrents;
        let query = format!(
            "UPDATE `{}` SET `{}`=0, `{}`=0",
            structure.table_name, structure.column_seeds, structure.column_peers
        );
        if let Err(e) = sqlx::query(sqlx::AssertSqlSafe(query)).execute(&mut *transaction).await {
            error!("{LOG_PREFIX} Error: {e}");
            return Err(e);
        }
        self.commit(transaction).await?;
        Ok(())
    }

    /// Commits and reopens the transaction every `chunk_size` rows.
    ///
    /// Commits the old transaction *before* opening the new one. The reverse
    /// order held two pool connections at once, so a handful of concurrent sync
    /// tasks could exhaust the pool and fail with an acquire timeout instead of
    /// the real error.
    async fn commit_chunk<'a>(&self, transaction: Transaction<'a, MySql>, in_chunk: &mut u64, chunk_size: u64) -> Result<Transaction<'a, MySql>, Error> {
        *in_chunk += 1;
        if chunk_size == 0 || *in_chunk < chunk_size {
            return Ok(transaction);
        }
        self.commit(transaction).await?;
        *in_chunk = 0;
        self.pool.begin().await.inspect_err(|e| error!("{LOG_PREFIX} Error: {e}"))
    }

    /// Commits the given transaction, logging and returning any failure.
    ///
    /// # Errors
    ///
    /// Returns the underlying `sqlx` error when the database operation fails.
    pub async fn commit(&self, transaction: Transaction<'_, MySql>) -> Result<(), Error> {
        match transaction.commit().await {
            Ok(()) => Ok(()),
            Err(e) => {
                error!("{LOG_PREFIX} Error: {e}");
                Err(e)
            }
        }
    }
}

#[async_trait]
impl DatabaseBackend for DatabaseConnectorMySQL {
    async fn load_torrents(&self, tracker: Arc<TorrentTracker>) -> Result<(u64, u64), Error> {
        DatabaseConnectorMySQL::load_torrents(self, tracker).await
    }

    async fn load_whitelist(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error> {
        DatabaseConnectorMySQL::load_whitelist(self, tracker).await
    }

    async fn load_blacklist(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error> {
        DatabaseConnectorMySQL::load_blacklist(self, tracker).await
    }

    async fn load_keys(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error> {
        DatabaseConnectorMySQL::load_keys(self, tracker).await
    }

    async fn load_users(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error> {
        DatabaseConnectorMySQL::load_users(self, tracker).await
    }

    async fn save_torrents(
        &self,
        tracker: Arc<TorrentTracker>,
        torrents: &BTreeMap<InfoHash, (TorrentUpdateData, UpdatesAction)>,
    ) -> Result<(), Error> {
        DatabaseConnectorMySQL::save_torrents(self, tracker, torrents).await
    }

    async fn save_whitelist(
        &self,
        tracker: Arc<TorrentTracker>,
        whitelists: Vec<(InfoHash, UpdatesAction)>,
    ) -> Result<u64, Error> {
        DatabaseConnectorMySQL::save_whitelist(self, tracker, whitelists).await
    }

    async fn save_blacklist(
        &self,
        tracker: Arc<TorrentTracker>,
        blacklists: Vec<(InfoHash, UpdatesAction)>,
    ) -> Result<u64, Error> {
        DatabaseConnectorMySQL::save_blacklist(self, tracker, blacklists).await
    }

    async fn save_keys(
        &self,
        tracker: Arc<TorrentTracker>,
        keys: BTreeMap<InfoHash, (i64, UpdatesAction)>,
    ) -> Result<u64, Error> {
        DatabaseConnectorMySQL::save_keys(self, tracker, keys).await
    }

    async fn save_users(
        &self,
        tracker: Arc<TorrentTracker>,
        users: BTreeMap<UserId, (UserEntryItem, UpdatesAction)>,
    ) -> Result<(), Error> {
        DatabaseConnectorMySQL::save_users(self, tracker, users).await
    }

    async fn reset_seeds_peers(&self, tracker: Arc<TorrentTracker>) -> Result<(), Error> {
        DatabaseConnectorMySQL::reset_seeds_peers(self, tracker).await
    }
}