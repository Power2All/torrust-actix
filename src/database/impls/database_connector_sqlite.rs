use crate::config::structs::configuration::Configuration;
use crate::database::enums::database_drivers::DatabaseDrivers;
use crate::database::helpers::{
    build_delete_hash_query, build_insert_ignore_hash_query, build_select_hash_query,
    build_update_ignore_torrent_query, build_upsert_torrent_query,
    limit_offset, upsert_conflict_clause,
};
use crate::database::structs::database_connector::DatabaseConnector;
use crate::database::structs::database_connector_sqlite::DatabaseConnectorSQLite;
use crate::database::traits::database_backend::DatabaseBackend;
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_entry::AHashMap;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::tracker::structs::user_entry_item::UserEntryItem;
use crate::tracker::structs::user_id::UserId;
use async_std::task;
use async_trait::async_trait;
use futures_util::TryStreamExt;
use log::{error, info};
use sha1::{Digest, Sha1};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{ConnectOptions, Error, Pool, Row, Sqlite, Transaction};
use std::collections::BTreeMap;
use std::process::exit;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

const ENGINE: DatabaseDrivers = DatabaseDrivers::sqlite3;
const LOG_PREFIX: &str = "[SQLite]";

impl DatabaseConnectorSQLite {
    #[tracing::instrument(level = "debug")]
    pub async fn create(dsl: &str) -> Result<Pool<Sqlite>, Error> {
        let options = SqliteConnectOptions::from_str(dsl)?
            .log_statements(log::LevelFilter::Debug)
            .log_slow_statements(log::LevelFilter::Debug, Duration::from_secs(1));
        SqlitePoolOptions::new()
            .connect_with(options.create_if_missing(true))
            .await
    }

    #[tracing::instrument(level = "debug")]
    pub async fn database_connector(
        config: Arc<Configuration>,
        create_database: bool,
    ) -> DatabaseConnector {
        let sqlite_connect =
            DatabaseConnectorSQLite::create(config.database.clone().path.as_str()).await;
        if let Err(sqlite_connect) = sqlite_connect {
            error!(
                "{} Unable to connect to SQLite on DSL {}",
                LOG_PREFIX,
                config.database.clone().path
            );
            error!(
                "{} Message: {:#?}",
                LOG_PREFIX,
                sqlite_connect.into_database_error().unwrap().message()
            );
            exit(1);
        }
        let mut structure = DatabaseConnector {
            mysql: None,
            sqlite: None,
            pgsql: None,
            engine: None,
        };
        structure.sqlite = Some(DatabaseConnectorSQLite {
            pool: sqlite_connect.unwrap(),
        });
        structure.engine = Some(DatabaseDrivers::sqlite3);
        if create_database {
            let pool = &structure.sqlite.clone().unwrap().pool;
            info!("[BOOT] Database creation triggered for SQLite.");
            info!("[BOOT SQLite] Setting the PRAGMA config...");
            let _ = sqlx::query("PRAGMA temp_store = memory;")
                .execute(pool)
                .await;
            let _ = sqlx::query("PRAGMA mmap_size = 30000000000;")
                .execute(pool)
                .await;
            let _ = sqlx::query("PRAGMA page_size = 32768;")
                .execute(pool)
                .await;
            let _ = sqlx::query("PRAGMA synchronous = full;")
                .execute(pool)
                .await;
            let ts = &config.database_structure.torrents;
            let hash_type = if ts.bin_type_infohash { "BLOB" } else { "TEXT" };
            info!("[BOOT SQLite] Creating table {}", ts.table_name);
            let query = format!(
                "CREATE TABLE IF NOT EXISTS `{}` (`{}` {} PRIMARY KEY NOT NULL, `{}` INTEGER DEFAULT 0, `{}` INTEGER DEFAULT 0, `{}` INTEGER DEFAULT 0)",
                ts.table_name, ts.column_infohash, hash_type, ts.column_seeds, ts.column_peers, ts.column_completed
            );
            if let Err(e) = sqlx::query(&query).execute(pool).await {
                panic!("{} Error: {}", LOG_PREFIX, e);
            }
            let ws = &config.database_structure.whitelist;
            let hash_type = if ws.bin_type_infohash { "BLOB" } else { "TEXT" };
            info!("[BOOT SQLite] Creating table {}", ws.table_name);
            let query = format!(
                "CREATE TABLE IF NOT EXISTS `{}` (`{}` {} PRIMARY KEY NOT NULL)",
                ws.table_name, ws.column_infohash, hash_type
            );
            if let Err(e) = sqlx::query(&query).execute(pool).await {
                panic!("{} Error: {}", LOG_PREFIX, e);
            }
            let bs = &config.database_structure.blacklist;
            let hash_type = if bs.bin_type_infohash { "BLOB" } else { "TEXT" };
            info!("[BOOT SQLite] Creating table {}", bs.table_name);
            let query = format!(
                "CREATE TABLE IF NOT EXISTS `{}` (`{}` {} PRIMARY KEY NOT NULL)",
                bs.table_name, bs.column_infohash, hash_type
            );
            if let Err(e) = sqlx::query(&query).execute(pool).await {
                panic!("{} Error: {}", LOG_PREFIX, e);
            }
            let ks = &config.database_structure.keys;
            let hash_type = if ks.bin_type_hash { "BLOB" } else { "TEXT" };
            info!("[BOOT SQLite] Creating table {}", ks.table_name);
            let query = format!(
                "CREATE TABLE IF NOT EXISTS `{}` (`{}` {} PRIMARY KEY NOT NULL, `{}` INTEGER DEFAULT 0)",
                ks.table_name, ks.column_hash, hash_type, ks.column_timeout
            );
            if let Err(e) = sqlx::query(&query).execute(pool).await {
                panic!("{} Error: {}", LOG_PREFIX, e);
            }
            let us = &config.database_structure.users;
            let key_type = if us.bin_type_key { "BLOB" } else { "TEXT" };
            info!("[BOOT SQLite] Creating table {}", us.table_name);
            let query = if us.id_uuid {
                format!(
                    "CREATE TABLE IF NOT EXISTS `{}` (`{}` TEXT PRIMARY KEY NOT NULL, `{}` {} NOT NULL, `{}` INTEGER NOT NULL DEFAULT 0, `{}` INTEGER NOT NULL DEFAULT 0, `{}` INTEGER NOT NULL DEFAULT 0, `{}` INTEGER NOT NULL DEFAULT 0, `{}` INTEGER NOT NULL DEFAULT 0)",
                    us.table_name, us.column_uuid, us.column_key, key_type, us.column_uploaded, us.column_downloaded, us.column_completed, us.column_active, us.column_updated
                )
            } else {
                format!(
                    "CREATE TABLE IF NOT EXISTS `{}` (`{}` INTEGER PRIMARY KEY AUTOINCREMENT, `{}` {} NOT NULL, `{}` INTEGER NOT NULL DEFAULT 0, `{}` INTEGER NOT NULL DEFAULT 0, `{}` INTEGER NOT NULL DEFAULT 0, `{}` INTEGER NOT NULL DEFAULT 0, `{}` INTEGER NOT NULL DEFAULT 0)",
                    us.table_name, us.column_id, us.column_key, key_type, us.column_uploaded, us.column_downloaded, us.column_completed, us.column_active, us.column_updated
                )
            };
            if let Err(e) = sqlx::query(&query).execute(pool).await {
                panic!("{} Error: {}", LOG_PREFIX, e);
            }
            info!("[BOOT] Created the database and tables, restart without the parameter to start the app.");
            task::sleep(Duration::from_secs(1)).await;
            exit(0);
        }
        structure
    }

    #[tracing::instrument(level = "debug")]
    pub async fn load_torrents(&self, tracker: Arc<TorrentTracker>) -> Result<(u64, u64), Error> {
        let mut start = 0u64;
        let length = 100000u64;
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
            let mut rows = sqlx::query(&query).fetch(&self.pool);
            while let Some(result) = rows.try_next().await? {
                let info_hash_data: &[u8] = result.get(structure.column_infohash.as_str());
                let info_hash: [u8; 20] =
                    <[u8; 20]>::try_from(hex::decode(info_hash_data).unwrap()[0..20].as_ref())
                        .unwrap();
                let completed_count: u32 = result.get(structure.column_completed.as_str());
                tracker.add_torrent(
                    InfoHash(info_hash),
                    TorrentEntry {
                        seeds: AHashMap::default(),
                        peers: AHashMap::default(),
                        completed: completed_count as u64,
                        updated: std::time::Instant::now(),
                    },
                );
                torrents += 1;
                completed += completed_count as u64;
            }
            start += length;
            if torrents < start {
                break;
            }
            info!("{} Handled {} torrents", LOG_PREFIX, torrents);
        }
        tracker.set_stats(StatsEvent::Completed, completed as i64);
        info!(
            "{} Loaded {} torrents with {} completed",
            LOG_PREFIX, torrents, completed
        );
        Ok((torrents, completed))
    }

    #[tracing::instrument(level = "debug")]
    pub async fn save_torrents(
        &self,
        tracker: Arc<TorrentTracker>,
        torrents: BTreeMap<InfoHash, (TorrentEntry, UpdatesAction)>,
    ) -> Result<(), Error> {
        let mut transaction = self.pool.begin().await?;
        let mut handled = 0u64;
        let structure = &tracker.config.database_structure.torrents;
        let db_config = &tracker.config.database;
        let is_binary = structure.bin_type_infohash;
        for (info_hash, (torrent_entry, updates_action)) in torrents.iter() {
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
                        if let Err(e) = sqlx::query(&query).execute(&mut *transaction).await {
                            error!("{} Error: {}", LOG_PREFIX, e);
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
                                    (&structure.column_seeds, &torrent_entry.seeds.len().to_string()),
                                    (&structure.column_peers, &torrent_entry.peers.len().to_string()),
                                ],
                                &[&structure.column_seeds, &structure.column_peers],
                                &hash_str,
                                is_binary,
                            );
                            if let Err(e) = sqlx::query(&query).execute(&mut *transaction).await {
                                error!("{} Error: {}", LOG_PREFIX, e);
                                return Err(e);
                            }
                        }
                        if db_config.update_completed {
                            let query = build_upsert_torrent_query(
                                ENGINE,
                                &structure.table_name,
                                &structure.column_infohash,
                                &[(&structure.column_completed, &torrent_entry.completed.to_string())],
                                &[&structure.column_completed],
                                &hash_str,
                                is_binary,
                            );
                            if let Err(e) = sqlx::query(&query).execute(&mut *transaction).await {
                                error!("{} Error: {}", LOG_PREFIX, e);
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
                                    (&structure.column_seeds, &torrent_entry.seeds.len().to_string()),
                                    (&structure.column_peers, &torrent_entry.peers.len().to_string()),
                                ],
                                &hash_str,
                                is_binary,
                            );
                            if let Err(e) = sqlx::query(&query).execute(&mut *transaction).await {
                                error!("{} Error: {}", LOG_PREFIX, e);
                                return Err(e);
                            }
                        }
                        if db_config.update_completed {
                            let query = build_update_ignore_torrent_query(
                                ENGINE,
                                &structure.table_name,
                                &structure.column_infohash,
                                &[(&structure.column_completed, &torrent_entry.completed.to_string())],
                                &hash_str,
                                is_binary,
                            );
                            if let Err(e) = sqlx::query(&query).execute(&mut *transaction).await {
                                error!("{} Error: {}", LOG_PREFIX, e);
                                return Err(e);
                            }
                        }
                    }
                }
            }
            if (handled as f64 / 1000f64).fract() == 0.0 || torrents.len() as u64 == handled {
                info!("{} Handled {} torrents", LOG_PREFIX, handled);
            }
        }
        info!("{} Handled {} torrents", LOG_PREFIX, handled);
        self.commit(transaction).await
    }

    #[tracing::instrument(level = "debug")]
    pub async fn load_whitelist(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error> {
        let mut start = 0u64;
        let length = 100000u64;
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
            let mut rows = sqlx::query(&query).fetch(&self.pool);
            while let Some(result) = rows.try_next().await? {
                let info_hash_data: &[u8] = result.get(structure.column_infohash.as_str());
                let info_hash: [u8; 20] =
                    <[u8; 20]>::try_from(hex::decode(info_hash_data).unwrap()[0..20].as_ref())
                        .unwrap();
                tracker.add_whitelist(InfoHash(info_hash));
                hashes += 1;
            }
            start += length;
            if hashes < start {
                break;
            }
            info!("{} Handled {} whitelisted torrents", LOG_PREFIX, hashes);
        }
        info!("{} Handled {} whitelisted torrents", LOG_PREFIX, hashes);
        Ok(hashes)
    }

    #[tracing::instrument(level = "debug")]
    pub async fn save_whitelist(
        &self,
        tracker: Arc<TorrentTracker>,
        whitelists: Vec<(InfoHash, UpdatesAction)>,
    ) -> Result<u64, Error> {
        let mut transaction = self.pool.begin().await?;
        let mut handled = 0u64;
        let structure = &tracker.config.database_structure.whitelist;
        let is_binary = structure.bin_type_infohash;
        for (info_hash, updates_action) in whitelists.iter() {
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
                        if let Err(e) = sqlx::query(&query).execute(&mut *transaction).await {
                            error!("{} Error: {}", LOG_PREFIX, e);
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
                    if let Err(e) = sqlx::query(&query).execute(&mut *transaction).await {
                        error!("{} Error: {}", LOG_PREFIX, e);
                        return Err(e);
                    }
                }
            }
            if (handled as f64 / 1000f64).fract() == 0.0 {
                info!("{} Handled {} whitelisted torrents", LOG_PREFIX, handled);
            }
        }
        info!("{} Handled {} whitelisted torrents", LOG_PREFIX, handled);
        let _ = self.commit(transaction).await;
        Ok(handled)
    }

    #[tracing::instrument(level = "debug")]
    pub async fn load_blacklist(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error> {
        let mut start = 0u64;
        let length = 100000u64;
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
            let mut rows = sqlx::query(&query).fetch(&self.pool);
            while let Some(result) = rows.try_next().await? {
                let info_hash_data: &[u8] = result.get(structure.column_infohash.as_str());
                let info_hash: [u8; 20] =
                    <[u8; 20]>::try_from(hex::decode(info_hash_data).unwrap()[0..20].as_ref())
                        .unwrap();
                tracker.add_blacklist(InfoHash(info_hash));
                hashes += 1;
            }
            start += length;
            if hashes < start {
                break;
            }
            info!("{} Handled {} blacklisted torrents", LOG_PREFIX, hashes);
        }
        info!("{} Handled {} blacklisted torrents", LOG_PREFIX, hashes);
        Ok(hashes)
    }

    #[tracing::instrument(level = "debug")]
    pub async fn save_blacklist(
        &self,
        tracker: Arc<TorrentTracker>,
        blacklists: Vec<(InfoHash, UpdatesAction)>,
    ) -> Result<u64, Error> {
        let mut transaction = self.pool.begin().await?;
        let mut handled = 0u64;
        let structure = &tracker.config.database_structure.blacklist;
        let is_binary = structure.bin_type_infohash;
        for (info_hash, updates_action) in blacklists.iter() {
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
                        if let Err(e) = sqlx::query(&query).execute(&mut *transaction).await {
                            error!("{} Error: {}", LOG_PREFIX, e);
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
                    if let Err(e) = sqlx::query(&query).execute(&mut *transaction).await {
                        error!("{} Error: {}", LOG_PREFIX, e);
                        return Err(e);
                    }
                }
            }
            if (handled as f64 / 1000f64).fract() == 0.0 {
                info!("{} Handled {} blacklisted torrents", LOG_PREFIX, handled);
            }
        }
        info!("{} Handled {} blacklisted torrents", LOG_PREFIX, handled);
        let _ = self.commit(transaction).await;
        Ok(handled)
    }

    #[tracing::instrument(level = "debug")]
    pub async fn load_keys(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error> {
        let mut start = 0u64;
        let length = 100000u64;
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
            let mut rows = sqlx::query(&query).fetch(&self.pool);
            while let Some(result) = rows.try_next().await? {
                let hash_data: &[u8] = result.get(structure.column_hash.as_str());
                let hash: [u8; 20] =
                    <[u8; 20]>::try_from(hex::decode(hash_data).unwrap()[0..20].as_ref()).unwrap();
                let timeout: i64 = result.get(structure.column_timeout.as_str());
                tracker.add_key(InfoHash(hash), timeout);
                hashes += 1;
            }
            start += length;
            if hashes < start {
                break;
            }
            info!("{} Handled {} keys", LOG_PREFIX, hashes);
        }
        info!("{} Handled {} keys", LOG_PREFIX, hashes);
        Ok(hashes)
    }

    #[tracing::instrument(level = "debug")]
    pub async fn save_keys(
        &self,
        tracker: Arc<TorrentTracker>,
        keys: BTreeMap<InfoHash, (i64, UpdatesAction)>,
    ) -> Result<u64, Error> {
        let mut transaction = self.pool.begin().await?;
        let mut handled = 0u64;
        let structure = &tracker.config.database_structure.keys;
        let is_binary = structure.bin_type_hash;
        for (hash, (timeout, update_action)) in keys.iter() {
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
                        if let Err(e) = sqlx::query(&query).execute(&mut *transaction).await {
                            error!("{} Error: {}", LOG_PREFIX, e);
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
                    if let Err(e) = sqlx::query(&query).execute(&mut *transaction).await {
                        error!("{} Error: {}", LOG_PREFIX, e);
                        return Err(e);
                    }
                }
            }
            if (handled as f64 / 1000f64).fract() == 0.0 {
                info!("{} Handled {} keys", LOG_PREFIX, handled);
            }
        }
        info!("{} Handled {} keys", LOG_PREFIX, handled);
        let _ = self.commit(transaction).await;
        Ok(handled)
    }

    #[tracing::instrument(level = "debug")]
    pub async fn load_users(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error> {
        let mut start = 0u64;
        let length = 100000u64;
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
            let mut rows = sqlx::query(&query).fetch(&self.pool);
            while let Some(result) = rows.try_next().await? {
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
                tracker.add_user(
                    UserId(hash),
                    UserEntryItem {
                        key: UserId::from_str(result.get(structure.column_key.as_str())).unwrap(),
                        user_id: if is_uuid {
                            None
                        } else {
                            Some(result.get::<u32, &str>(structure.column_id.as_str()) as u64)
                        },
                        user_uuid: if is_uuid {
                            Some(result.get(structure.column_uuid.as_str()))
                        } else {
                            None
                        },
                        uploaded: result.get::<u32, &str>(structure.column_uploaded.as_str()) as u64,
                        downloaded: result.get::<u32, &str>(structure.column_downloaded.as_str())
                            as u64,
                        completed: result.get::<u32, &str>(structure.column_completed.as_str())
                            as u64,
                        updated: result.get::<u32, &str>(structure.column_updated.as_str()) as u64,
                        active: result.get::<i8, &str>(structure.column_active.as_str()) as u8,
                        torrents_active: Default::default(),
                    },
                );
                hashes += 1;
            }
            start += length;
            if hashes < start {
                break;
            }
            info!("{} Handled {} users", LOG_PREFIX, hashes);
        }
        info!("{} Handled {} users", LOG_PREFIX, hashes);
        Ok(hashes)
    }

    #[tracing::instrument(level = "debug")]
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
        for (_, (user_entry_item, updates_action)) in users.iter() {
            handled += 1;
            match updates_action {
                UpdatesAction::Remove => {
                    if db_config.remove_action {
                        let query = if is_uuid {
                            format!(
                                "DELETE FROM `{}` WHERE `{}`='{}'",
                                structure.table_name,
                                structure.column_uuid,
                                user_entry_item.user_uuid.clone().unwrap()
                            )
                        } else {
                            format!(
                                "DELETE FROM `{}` WHERE `{}`='{}'",
                                structure.table_name,
                                structure.column_id,
                                user_entry_item.user_id.unwrap()
                            )
                        };
                        if let Err(e) = sqlx::query(&query).execute(&mut *transaction).await {
                            error!("{} Error: {}", LOG_PREFIX, e);
                            return Err(e);
                        }
                    }
                }
                UpdatesAction::Add | UpdatesAction::Update => {
                    let key_value = if is_binary_key {
                        format!("X'{}'", user_entry_item.key)
                    } else {
                        format!("'{}'", user_entry_item.key)
                    };

                    let query = if db_config.insert_vacant {
                        let (id_col, id_val) = if is_uuid {
                            (
                                &structure.column_uuid,
                                format!("'{}'", user_entry_item.user_uuid.clone().unwrap()),
                            )
                        } else {
                            (
                                &structure.column_id,
                                format!("{}", user_entry_item.user_id.unwrap()),
                            )
                        };
                        let conflict_clause = upsert_conflict_clause(
                            ENGINE,
                            id_col,
                            &[
                                &structure.column_completed,
                                &structure.column_active,
                                &structure.column_downloaded,
                                &structure.column_key,
                                &structure.column_uploaded,
                                &structure.column_updated,
                            ],
                        );
                        format!(
                            "INSERT INTO `{}` (`{}`, `{}`, `{}`, `{}`, `{}`, `{}`, `{}`) VALUES ({}, {}, {}, {}, {}, {}, {}) {}",
                            structure.table_name,
                            id_col,
                            structure.column_completed,
                            structure.column_active,
                            structure.column_downloaded,
                            structure.column_key,
                            structure.column_uploaded,
                            structure.column_updated,
                            id_val,
                            user_entry_item.completed,
                            user_entry_item.active,
                            user_entry_item.downloaded,
                            key_value,
                            user_entry_item.uploaded,
                            user_entry_item.updated,
                            conflict_clause
                        )
                    } else {
                        let (where_col, where_val) = if is_uuid {
                            (
                                &structure.column_uuid,
                                format!("'{}'", user_entry_item.user_uuid.clone().unwrap()),
                            )
                        } else {
                            (
                                &structure.column_id,
                                format!("{}", user_entry_item.user_id.unwrap()),
                            )
                        };
                        format!(
                            "UPDATE OR IGNORE `{}` SET `{}`={}, `{}`={}, `{}`={}, `{}`={}, `{}`={}, `{}`={} WHERE `{}`={}",
                            structure.table_name,
                            structure.column_completed,
                            user_entry_item.completed,
                            structure.column_active,
                            user_entry_item.active,
                            structure.column_downloaded,
                            user_entry_item.downloaded,
                            structure.column_key,
                            key_value,
                            structure.column_uploaded,
                            user_entry_item.uploaded,
                            structure.column_updated,
                            user_entry_item.updated,
                            where_col,
                            where_val
                        )
                    };
                    if let Err(e) = sqlx::query(&query).execute(&mut *transaction).await {
                        error!("{} Error: {}", LOG_PREFIX, e);
                        return Err(e);
                    }
                }
            }
            if (handled as f64 / 1000f64).fract() == 0.0 || users.len() as u64 == handled {
                info!("{} Handled {} users", LOG_PREFIX, handled);
            }
        }
        info!("{} Handled {} users", LOG_PREFIX, handled);
        self.commit(transaction).await
    }

    #[tracing::instrument(level = "debug")]
    pub async fn reset_seeds_peers(&self, tracker: Arc<TorrentTracker>) -> Result<(), Error> {
        let mut transaction = self.pool.begin().await?;
        let structure = &tracker.config.database_structure.torrents;
        let query = format!(
            "UPDATE `{}` SET `{}`=0, `{}`=0",
            structure.table_name, structure.column_seeds, structure.column_peers
        );
        if let Err(e) = sqlx::query(&query).execute(&mut *transaction).await {
            error!("{} Error: {}", LOG_PREFIX, e);
            return Err(e);
        }
        let _ = self.commit(transaction).await;
        Ok(())
    }

    #[tracing::instrument(level = "debug")]
    pub async fn commit(&self, transaction: Transaction<'_, Sqlite>) -> Result<(), Error> {
        match transaction.commit().await {
            Ok(_) => Ok(()),
            Err(e) => {
                error!("{} Error: {}", LOG_PREFIX, e);
                Err(e)
            }
        }
    }
}

#[async_trait]
impl DatabaseBackend for DatabaseConnectorSQLite {
    async fn load_torrents(&self, tracker: Arc<TorrentTracker>) -> Result<(u64, u64), Error> {
        DatabaseConnectorSQLite::load_torrents(self, tracker).await
    }

    async fn load_whitelist(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error> {
        DatabaseConnectorSQLite::load_whitelist(self, tracker).await
    }

    async fn load_blacklist(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error> {
        DatabaseConnectorSQLite::load_blacklist(self, tracker).await
    }

    async fn load_keys(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error> {
        DatabaseConnectorSQLite::load_keys(self, tracker).await
    }

    async fn load_users(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error> {
        DatabaseConnectorSQLite::load_users(self, tracker).await
    }

    async fn save_torrents(
        &self,
        tracker: Arc<TorrentTracker>,
        torrents: BTreeMap<InfoHash, (TorrentEntry, UpdatesAction)>,
    ) -> Result<(), Error> {
        DatabaseConnectorSQLite::save_torrents(self, tracker, torrents).await
    }

    async fn save_whitelist(
        &self,
        tracker: Arc<TorrentTracker>,
        whitelists: Vec<(InfoHash, UpdatesAction)>,
    ) -> Result<u64, Error> {
        DatabaseConnectorSQLite::save_whitelist(self, tracker, whitelists).await
    }

    async fn save_blacklist(
        &self,
        tracker: Arc<TorrentTracker>,
        blacklists: Vec<(InfoHash, UpdatesAction)>,
    ) -> Result<u64, Error> {
        DatabaseConnectorSQLite::save_blacklist(self, tracker, blacklists).await
    }

    async fn save_keys(
        &self,
        tracker: Arc<TorrentTracker>,
        keys: BTreeMap<InfoHash, (i64, UpdatesAction)>,
    ) -> Result<u64, Error> {
        DatabaseConnectorSQLite::save_keys(self, tracker, keys).await
    }

    async fn save_users(
        &self,
        tracker: Arc<TorrentTracker>,
        users: BTreeMap<UserId, (UserEntryItem, UpdatesAction)>,
    ) -> Result<(), Error> {
        DatabaseConnectorSQLite::save_users(self, tracker, users).await
    }

    async fn reset_seeds_peers(&self, tracker: Arc<TorrentTracker>) -> Result<(), Error> {
        DatabaseConnectorSQLite::reset_seeds_peers(self, tracker).await
    }
}