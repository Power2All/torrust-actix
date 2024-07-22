use std::collections::{BTreeMap, HashMap};
use std::process::exit;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use futures_util::TryStreamExt;
use log::{error, info};
use regex::Regex;
use sqlx::{ConnectOptions, Error, Pool, Row, Sqlite, Transaction};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use crate::config::structs::configuration::Configuration;
use crate::database::enums::database_drivers::DatabaseDrivers;
use crate::database::structs::database_connector::DatabaseConnector;
use crate::database::structs::database_connector_sqlite::DatabaseConnectorSQLite;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::tracker::structs::user_entry_item::UserEntryItem;
use crate::tracker::structs::user_id::UserId;

impl DatabaseConnectorSQLite {
    pub async fn create(dsl: &str) -> Result<Pool<Sqlite>, Error>
    {
        let mut options = SqliteConnectOptions::from_str(dsl)?;
        options = options
            .log_statements(log::LevelFilter::Debug)
            .clone()
            .log_slow_statements(log::LevelFilter::Debug, Duration::from_secs(1))
            .clone();
        SqlitePoolOptions::new().connect_with(options.create_if_missing(true)).await
    }

    pub async fn database_connector(config: Arc<Configuration>) -> DatabaseConnector
    {
        let sqlite_connect = DatabaseConnectorSQLite::create(&config.db_path).await;
        if sqlite_connect.is_err() {
            error!("[SQLite] Unable to open the database {}", &config.db_path);
            error!("[SQLite] Message: {:#?}", sqlite_connect.unwrap_err().into_database_error());
            exit(1);
        }

        let mut structure = DatabaseConnector { mysql: None, sqlite: None, pgsql: None, engine: None };
        structure.sqlite = Some(DatabaseConnectorSQLite { pool: sqlite_connect.unwrap() });
        structure.engine = Some(DatabaseDrivers::sqlite3);

        let pool = &structure.sqlite.clone().unwrap().pool;
        let _ = sqlx::query("PRAGMA temp_store = memory;").execute(pool).await;
        let _ = sqlx::query("PRAGMA mmap_size = 30000000000;").execute(pool).await;
        let _ = sqlx::query("PRAGMA page_size = 32768;").execute(pool).await;
        let _ = sqlx::query("PRAGMA journal_mode = TRUNCATE;").execute(pool).await;
        let _ = sqlx::query("PRAGMA journal_size_limit = 536870912;").execute(pool).await;
        let _ = sqlx::query("PRAGMA synchronous = full;").execute(pool).await;
        let _ = sqlx::query(
            format!(
                "CREATE TABLE IF NOT EXISTS {} ({} VARCHAR(40) PRIMARY KEY, {} INTEGER DEFAULT 0 NOT NULL)",
                config.db_structure.db_torrents,
                config.db_structure.table_torrents_info_hash,
                config.db_structure.table_torrents_completed
            ).as_str()
        ).execute(pool).await;
        let _ = sqlx::query(
            format!(
                "CREATE TABLE IF NOT EXISTS {} ({} VARCHAR(40) PRIMARY KEY)",
                config.db_structure.db_whitelist,
                config.db_structure.table_whitelist_info_hash
            ).as_str()
        ).execute(pool).await;
        let _ = sqlx::query(
            format!(
                "CREATE TABLE IF NOT EXISTS {} ({} VARCHAR(40) PRIMARY KEY)",
                config.db_structure.db_blacklist,
                config.db_structure.table_blacklist_info_hash,
            ).as_str()
        ).execute(pool).await;
        let _ = sqlx::query(
            format!(
                "CREATE TABLE IF NOT EXISTS {} ({} VARCHAR(40) PRIMARY KEY, {} INTEGER DEFAULT 0 NOT NULL)",
                config.db_structure.db_keys,
                config.db_structure.table_keys_hash,
                config.db_structure.table_keys_timeout,
            ).as_str()
        ).execute(pool).await;
        let _ = sqlx::query(
            format!(
                "CREATE TABLE IF NOT EXISTS {} ({} VARCHAR(36) PRIMARY KEY, {} VARCHAR(40) NULL, {} INTEGER DEFAULT 0 NOT NULL, {} INTEGER DEFAULT 0 NOT NULL, {} INTEGER DEFAULT 0 NOT NULL, {} INTEGER DEFAULT 0 NOT NULL, {} INTEGER DEFAULT 0 NOT NULL)",
                config.db_structure.db_users,
                config.db_structure.table_users_uuid,
                config.db_structure.table_users_key,
                config.db_structure.table_users_uploaded,
                config.db_structure.table_users_downloaded,
                config.db_structure.table_users_completed,
                config.db_structure.table_users_updated,
                config.db_structure.table_users_active,
            ).as_str()
        ).execute(pool).await;

        structure
    }

    pub async fn load_torrents(&self, tracker: Arc<TorrentTracker>) -> Result<(u64, u64), Error>
    {
        let mut total_torrents = 0u64;
        let mut total_completes = 0u64;
        let query = format!(
            "SELECT {},{} FROM {}",
            tracker.config.db_structure.table_torrents_info_hash,
            tracker.config.db_structure.table_torrents_completed,
            tracker.config.db_structure.db_torrents
        );
        let mut rows = sqlx::query(query.as_str()).fetch(&self.pool);
        while let Some(result) = rows.try_next().await? {
            let info_hash_data: &str = result.get(tracker.config.db_structure.table_torrents_info_hash.clone().as_str());
            let info_hash_decoded = hex::decode(info_hash_data).unwrap();
            let completed_data: i64 = result.get(tracker.config.db_structure.table_torrents_completed.clone().as_str());
            let info_hash = InfoHash(<[u8; 20]>::try_from(info_hash_decoded[0..20].as_ref()).unwrap());
            tracker.add_torrent(info_hash, TorrentEntry {
                peers: BTreeMap::new(),
                seeds: BTreeMap::new(),
                completed: completed_data as u64,
                updated: std::time::Instant::now(),
            });
            total_torrents += 1;
            total_completes += completed_data as u64;
        }
        info!("[SQLite3] Loaded {} torrents...", total_torrents);
        Ok((total_torrents, total_completes))
    }

    pub async fn save_torrents(&self, tracker: Arc<TorrentTracker>, torrents: HashMap<InfoHash, i64>) -> Result<(), Error>
    {
        let mut torrents_transaction = self.pool.begin().await?;
        let mut torrents_handled_entries = 0u64;
        for (info_hash, completed) in torrents.iter() {
            torrents_handled_entries += 1;
            match sqlx::query(&format!(
                "INSERT OR REPLACE INTO {} ({},{}) VALUES ('{}',{})",
                tracker.config.db_structure.db_torrents,
                tracker.config.db_structure.table_torrents_info_hash,
                tracker.config.db_structure.table_torrents_completed,
                info_hash,
                completed.clone()
            ))
                .execute(&mut *torrents_transaction)
                .await {
                Ok(_) => {}
                Err(e) => {
                    error!("[SQLite3] Error: {}", e.to_string());
                    return Err(e);
                }
            }

            if (torrents_handled_entries as f64 / 10000f64).fract() == 0.0 || torrents.len() as u64 == torrents_handled_entries {
                match torrents_transaction.commit().await {
                    Ok(_) => {}
                    Err(e) => {
                        error!("[SQLite3] Error: {}", e.to_string());
                        return Err(e);
                    }
                };
                info!("[SQLite3] Handled {} torrents", torrents_handled_entries);
                torrents_transaction = self.pool.begin().await?
            }
        }
        self.commit(torrents_transaction).await
    }

    pub async fn load_whitelist(&self, tracker: Arc<TorrentTracker>) -> Result<Vec<InfoHash>, Error>
    {
        let mut return_data_whitelist = vec![];
        let mut counter = 0u64;
        let mut total_whitelist = 0u64;

        let query = format!(
            "SELECT {} FROM {}",
            tracker.config.db_structure.table_whitelist_info_hash,
            tracker.config.db_structure.db_whitelist
        );
        let mut rows = sqlx::query(query.as_str()).fetch(&self.pool);
        while let Some(result) = rows.try_next().await? {
            if counter == 100000 {
                info!("[SQLite3] Loaded {} whitelists...", total_whitelist);
                counter = 0;
            }
            let info_hash_data: &str = result.get(tracker.config.db_structure.table_whitelist_info_hash.clone().as_str());
            let info_hash_decoded = hex::decode(info_hash_data).unwrap();
            let info_hash = <[u8; 20]>::try_from(info_hash_decoded[0..20].as_ref()).unwrap();
            return_data_whitelist.push(InfoHash(info_hash));
            counter += 1;
            total_whitelist += 1;
        }

        info!("[SQLite3] Loaded {} whitelists...", total_whitelist);
        Ok(return_data_whitelist)
    }

    pub async fn save_whitelist(&self, tracker: Arc<TorrentTracker>, whitelists: Vec<(InfoHash, i64)>) -> Result<(), Error>
    {
        let mut whitelist_transaction = self.pool.begin().await?;
        let mut whitelist_handled_entries = 0u64;
        for (info_hash, value) in whitelists.iter() {
            if value == &2 {
                whitelist_handled_entries += 1;
                match sqlx::query(&format!(
                    "INSERT OR IGNORE INTO {} ({}) VALUES ('{}')",
                    tracker.config.db_structure.db_whitelist,
                    tracker.config.db_structure.table_whitelist_info_hash,
                    info_hash
                ))
                    .execute(&mut *whitelist_transaction)
                    .await {
                    Ok(_) => {}
                    Err(e) => {
                        error!("[SQLite3] Error: {}", e.to_string());
                        return Err(e);
                    }
                }

                if (whitelist_handled_entries as f64 / 1000f64).fract() == 0.0 || whitelists.len() as u64 == whitelist_handled_entries {
                    info!("[SQLite3] Handled {} whitelists", whitelist_handled_entries);
                }
            }
            if value == &0 {
                match sqlx::query(&format!(
                    "DELETE FROM {} WHERE {} = '{}'",
                    tracker.config.db_structure.db_whitelist,
                    tracker.config.db_structure.table_whitelist_info_hash,
                    info_hash
                ))
                    .execute(&mut *whitelist_transaction)
                    .await {
                    Ok(_) => {}
                    Err(e) => {
                        error!("[SQLite3] Error: {}", e.to_string());
                        return Err(e);
                    }
                }
            }
        }
        self.commit(whitelist_transaction).await
    }

    pub async fn load_blacklist(&self, tracker: Arc<TorrentTracker>) -> Result<Vec<InfoHash>, Error>
    {
        let mut return_data_blacklist = vec![];
        let mut counter = 0u64;
        let mut total_blacklist = 0u64;

        let query = format!(
            "SELECT {} FROM {}",
            tracker.config.db_structure.table_blacklist_info_hash,
            tracker.config.db_structure.db_blacklist
        );
        let mut rows = sqlx::query(query.as_str()).fetch(&self.pool);
        while let Some(result) = rows.try_next().await? {
            if counter == 100000 {
                info!("[SQLite3] Loaded {} blacklists...", total_blacklist);
                counter = 0;
            }
            let info_hash_data: &str = result.get(tracker.config.db_structure.table_blacklist_info_hash.clone().as_str());
            let info_hash_decoded = hex::decode(info_hash_data).unwrap();
            let info_hash = <[u8; 20]>::try_from(info_hash_decoded[0..20].as_ref()).unwrap();
            return_data_blacklist.push(InfoHash(info_hash));
            counter += 1;
            total_blacklist += 1;
        }

        info!("[SQLite3] Loaded {} blacklists...", total_blacklist);
        Ok(return_data_blacklist)
    }

    pub async fn save_blacklist(&self, tracker: Arc<TorrentTracker>, blacklists: Vec<InfoHash>) -> Result<(), Error>
    {
        let mut blacklist_transaction = self.pool.begin().await?;
        let mut blacklist_handled_entries = 0u64;
        for info_hash in blacklists.iter() {
            blacklist_handled_entries += 1;
            match sqlx::query(&format!(
                "INSERT OR REPLACE INTO {} ({}) VALUES ('{}')",
                tracker.config.db_structure.db_blacklist,
                tracker.config.db_structure.table_blacklist_info_hash,
                info_hash
            ))
                .execute(&mut *blacklist_transaction)
                .await {
                Ok(_) => {}
                Err(e) => {
                    error!("[SQLite3] Error: {}", e.to_string());
                    return Err(e);
                }
            }

            if (blacklist_handled_entries as f64 / 1000f64).fract() == 0.0 || blacklists.len() as u64 == blacklist_handled_entries {
                info!("[SQLite3] Handled {} whitelists", blacklist_handled_entries);
            }
        }
        self.commit(blacklist_transaction).await
    }

    pub async fn load_keys(&self, tracker: Arc<TorrentTracker>) -> Result<Vec<(InfoHash, i64)>, Error>
    {
        let mut return_data_keys = vec![];
        let mut counter = 0u64;
        let mut total_keys = 0u64;

        let query = format!(
            "SELECT {},{} FROM {}",
            tracker.config.db_structure.table_keys_hash,
            tracker.config.db_structure.table_keys_timeout,
            tracker.config.db_structure.db_keys
        );
        let mut rows = sqlx::query(query.as_str()).fetch(&self.pool);
        while let Some(result) = rows.try_next().await? {
            if counter == 100000 {
                info!("[SQLite3] Loaded {} keys...", total_keys);
                counter = 0;
            }
            let hash_data: &str = result.get(tracker.config.db_structure.table_keys_hash.clone().as_str());
            let hash_decoded = hex::decode(hash_data).unwrap();
            let timeout_data: i64 = result.get(tracker.config.db_structure.table_keys_timeout.clone().as_str());
            let hash = <[u8; 20]>::try_from(hash_decoded[0..20].as_ref()).unwrap();
            return_data_keys.push((InfoHash(hash), timeout_data));
            counter += 1;
            total_keys += 1;
        }

        info!("[SQLite3] Loaded {} keys...", total_keys);
        Ok(return_data_keys)
    }

    pub async fn save_keys(&self, tracker: Arc<TorrentTracker>, keys: Vec<(InfoHash, i64)>) -> Result<(), Error>
    {
        let mut keys_transaction = self.pool.begin().await?;
        let mut keys_handled_entries = 0u64;
        for (hash, timeout) in keys.iter() {
            keys_handled_entries += 1;
            match sqlx::query(&format!(
                "INSERT OR REPLACE INTO {} ({},{}) VALUES ('{}',{})",
                tracker.config.db_structure.db_keys,
                tracker.config.db_structure.table_keys_hash,
                tracker.config.db_structure.table_keys_timeout,
                hash,
                timeout.clone()
            ))
                .execute(&mut *keys_transaction)
                .await {
                Ok(_) => {}
                Err(e) => {
                    error!("[SQLite3] Error: {}", e.to_string());
                    return Err(e);
                }
            }

            if (keys_handled_entries as f64 / 1000f64).fract() == 0.0 || keys.len() as u64 == keys_handled_entries {
                info!("[SQLite3] Handled {} keys", keys_handled_entries);
            }
        }
        self.commit(keys_transaction).await
    }

    pub async fn load_users(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error>
    {
        let mut counter = 0u64;
        let mut total_users = 0u64;

        let query = format!(
            "SELECT {},{},{},{},{},{},{} FROM {}",
            tracker.config.db_structure.table_users_uuid,
            tracker.config.db_structure.table_users_key,
            tracker.config.db_structure.table_users_uploaded,
            tracker.config.db_structure.table_users_downloaded,
            tracker.config.db_structure.table_users_completed,
            tracker.config.db_structure.table_users_updated,
            tracker.config.db_structure.table_users_active,
            tracker.config.db_structure.db_torrents
        );
        let mut rows = sqlx::query(query.as_str()).fetch(&self.pool);
        let mut users_parsing = HashMap::new();
        while let Some(result) = rows.try_next().await? {
            if counter == 100000 {
                tracker.add_users(users_parsing.clone(), false).await;
                users_parsing.clear();
                info!("[SQLite3] Loaded {} users...", total_users);
                counter = 0;
            }

            let uuid_regex = Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$").unwrap();
            if !uuid_regex.is_match(result.get(tracker.config.db_structure.table_users_uuid.clone().to_lowercase().as_str())) {
                info!("[SQLite3] Could not parse the user with ID: {}", result.get::<&str, _>(tracker.config.db_structure.table_users_uuid.clone().to_lowercase().as_str()));
                continue;
            }
            let uuid: &str = result.get(tracker.config.db_structure.table_users_uuid.clone().to_lowercase().as_str());

            let user_key_data: &str = result.get(tracker.config.db_structure.table_users_key.clone().as_str());
            let user_key_decoded = hex::decode(user_key_data).unwrap();
            let user_key = <[u8; 20]>::try_from(user_key_decoded[0..20].as_ref()).unwrap();

            let uploaded: i64 = result.get(tracker.config.db_structure.table_users_uploaded.clone().as_str());
            let downloaded: i64 = result.get(tracker.config.db_structure.table_users_uploaded.clone().as_str());
            let completed: i64 = result.get(tracker.config.db_structure.table_users_completed.clone().as_str());
            let updated: i64 = result.get(tracker.config.db_structure.table_users_updated.clone().as_str());
            let active: i64 = result.get(tracker.config.db_structure.table_users_active.clone().as_str());

            users_parsing.insert(
                UserId(user_key),
                UserEntryItem {
                    uuid: uuid.to_string(),
                    key: UserId(user_key),
                    uploaded: uploaded as u64,
                    downloaded: downloaded as u64,
                    completed: completed as u64,
                    updated: updated as u64,
                    active: active as u8,
                    torrents_active: HashMap::new()
                }
            );
            counter += 1;
            total_users += 1;
        }

        if counter != 0 {
            tracker.add_users(users_parsing.clone(), false).await;
            users_parsing.clear();
        }

        info!("[SQLite3] Loaded {} users...", total_users);
        Ok(total_users)
    }

    pub async fn save_users(&self, tracker: Arc<TorrentTracker>, users: HashMap<UserId, UserEntryItem>) -> Result<(), Error>
    {
        let mut users_transaction = self.pool.begin().await?;
        let mut users_handled_entries = 0u64;
        for (_, user_entry_item) in users.iter() {
            match sqlx::query(&format!(
                "INSERT OR REPLACE INTO {} ('{}','{}',{},{},{},{},{}) VALUES ('{}','{}',{},{},{},{},{})",
                tracker.config.db_structure.db_users,
                tracker.config.db_structure.table_users_uuid,
                tracker.config.db_structure.table_users_key,
                tracker.config.db_structure.table_users_uploaded,
                tracker.config.db_structure.table_users_downloaded,
                tracker.config.db_structure.table_users_completed,
                tracker.config.db_structure.table_users_updated,
                tracker.config.db_structure.table_users_active,
                user_entry_item.uuid,
                user_entry_item.key,
                user_entry_item.uploaded,
                user_entry_item.downloaded,
                user_entry_item.completed,
                user_entry_item.updated,
                user_entry_item.active
            ))
                .execute(&mut *users_transaction)
                .await {
                Ok(_) => {}
                Err(e) => {
                    error!("[SQLite3] Error: {}", e.to_string());
                    return Err(e);
                }
            }
            users_handled_entries += 1;

            if (users_handled_entries as f64 / 10000f64).fract() == 0.0 || users.len() as u64 == users_handled_entries {
                match self.commit(users_transaction).await {
                    Ok(_) => {}
                    Err(e) => {
                        return Err(e);
                    }
                };
                info!("[SQLite3] Handled {} torrents", users_handled_entries);
                users_transaction = self.pool.begin().await?
            }
        }
        self.commit(users_transaction).await
    }

    pub async fn commit(&self, transaction: Transaction<'_, Sqlite>) -> Result<(), Error>
    {
        match transaction.commit().await {
            Ok(_) => {
                Ok(())
            }
            Err(e) => {
                error!("[SQLite3] Error: {}", e.to_string());
                Err(e)
            }
        }
    }
}
