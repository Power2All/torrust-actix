use clap::ValueEnum;
use futures::TryStreamExt;
use log::{info, error};
use scc::ebr::Arc;
use serde::{Deserialize, Serialize};
use sqlx::{Error, MySql, Pool, Postgres, Row, Sqlite, ConnectOptions};
use sqlx::mysql::{MySqlConnectOptions, MySqlPoolOptions};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::collections::HashMap;
use std::process::exit;
use std::str::FromStr;
use std::time::Duration;

use crate::common::InfoHash;
use crate::config::Configuration;
use crate::tracker::TorrentTracker;
use crate::tracker_objects::torrents::TorrentEntryItem;

#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum DatabaseDrivers {
    sqlite3,
    mysql,
    pgsql,
}

#[derive(Clone)]
pub struct DatabaseConnectorMySQL {
    pool: Pool<MySql>,
}

#[derive(Clone)]
pub struct DatabaseConnectorSQLite {
    pool: Pool<Sqlite>,
}

#[derive(Clone)]
pub struct DatabaseConnectorPgSQL {
    pool: Pool<Postgres>,
}

#[derive(Clone)]
pub struct DatabaseConnector {
    config: Arc<Configuration>,
    mysql: Option<DatabaseConnectorMySQL>,
    sqlite: Option<DatabaseConnectorSQLite>,
    pgsql: Option<DatabaseConnectorPgSQL>,
    engine: Option<DatabaseDrivers>,
}

impl DatabaseConnectorSQLite {
    pub async fn create(dsl: &str) -> Result<Pool<Sqlite>, Error>
    {
        let mut options = SqliteConnectOptions::from_str(dsl)?;
        options
            .log_statements(log::LevelFilter::Debug)
            .log_slow_statements(log::LevelFilter::Debug, Duration::from_secs(1));
        SqlitePoolOptions::new().connect_with(options.create_if_missing(true)).await
    }
}

impl DatabaseConnectorMySQL {
    pub async fn create(dsl: &str) -> Result<Pool<MySql>, Error>
    {
        let mut options = MySqlConnectOptions::from_str(dsl)?;
        options
            .log_statements(log::LevelFilter::Debug)
            .log_slow_statements(log::LevelFilter::Debug, Duration::from_secs(1));
        MySqlPoolOptions::new().connect_with(options).await
    }
}

impl DatabaseConnectorPgSQL {
    pub async fn create(dsl: &str) -> Result<Pool<Postgres>, Error>
    {
        let mut options = PgConnectOptions::from_str(dsl)?;
        options
            .log_statements(log::LevelFilter::Debug)
            .log_slow_statements(log::LevelFilter::Debug, Duration::from_secs(1));
        PgPoolOptions::new().connect_with(options).await
    }
}

impl DatabaseConnector {
    pub async fn new(config: Arc<Configuration>) -> DatabaseConnector
    {
        let mut structure = DatabaseConnector {
            config: config.clone(),
            mysql: None,
            sqlite: None,
            pgsql: None,
            engine: None,
        };

        match &config.db_driver {
            DatabaseDrivers::sqlite3 => {
                let sqlite_connect = DatabaseConnectorSQLite::create(&config.db_path).await;
                if sqlite_connect.is_err() {
                    error!("[SQLite] Unable to open the database {}", &config.db_path);
                    error!("[SQLite] Message: {:#?}", sqlite_connect.unwrap_err().into_database_error());
                    exit(1);
                }
                structure.sqlite = Some(DatabaseConnectorSQLite {
                    pool: sqlite_connect.unwrap()
                });
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
            }
            DatabaseDrivers::mysql => {
                let mysql_connect = DatabaseConnectorMySQL::create(&config.db_path).await;
                if mysql_connect.is_err() {
                    error!("[MySQL] Unable to connect to MySQL on DSL {}", &config.db_path);
                    exit(1);
                }
                structure.mysql = Some(DatabaseConnectorMySQL {
                    pool: mysql_connect.unwrap()
                });
                structure.engine = Some(DatabaseDrivers::mysql);
            }
            DatabaseDrivers::pgsql => {
                let pgsql_connect = DatabaseConnectorPgSQL::create(&config.db_path).await;
                if pgsql_connect.is_err() {
                    error!("[PgSQL] Unable to connect to PostgresSQL on DSL {}", &config.db_path)
                }
                structure.pgsql = Some(DatabaseConnectorPgSQL {
                    pool: pgsql_connect.unwrap()
                });
                structure.engine = Some(DatabaseDrivers::pgsql);
            }
        }

        structure
    }

    pub async fn load_whitelist(&self) -> Result<Vec<InfoHash>, Error>
    {
        let mut return_data_whitelist = vec![];
        let mut counter = 0u64;
        let mut total_whitelist = 0u64;

        if self.engine.is_some() {
            return match self.engine.clone().unwrap() {
                DatabaseDrivers::sqlite3 => {
                    let pool = &self.sqlite.clone().unwrap().pool;

                    let query = format!(
                        "SELECT {} FROM {}",
                        self.config.db_structure.table_whitelist_info_hash,
                        self.config.db_structure.db_whitelist
                    );
                    let mut rows = sqlx::query(
                        query.as_str()
                    ).fetch(pool);
                    while let Some(result) = rows.try_next().await? {
                        if counter == 100000 {
                            info!("[SQLite3] Loaded {} whitelists...", total_whitelist);
                            counter = 0;
                        }
                        let info_hash_data: &str = result.get(self.config.db_structure.table_whitelist_info_hash.clone().as_str());
                        let info_hash_decoded = hex::decode(info_hash_data).unwrap();
                        let info_hash = <[u8; 20]>::try_from(info_hash_decoded[0..20].as_ref()).unwrap();
                        return_data_whitelist.push(InfoHash(info_hash));
                        counter += 1;
                        total_whitelist += 1;
                    }

                    info!("[SQLite3] Loaded {} whitelists...", total_whitelist);
                    Ok(return_data_whitelist)
                }
                DatabaseDrivers::mysql => {
                    let pool = &self.mysql.clone().unwrap().pool;

                    let query = format!(
                        "SELECT `{}` FROM `{}`",
                        self.config.db_structure.table_whitelist_info_hash,
                        self.config.db_structure.db_whitelist
                    );
                    let mut rows = sqlx::query(
                        query.as_str()
                    ).fetch(pool);
                    while let Some(result) = rows.try_next().await? {
                        if counter == 100000 {
                            info!("[MySQL] Loaded {} whitelists...", total_whitelist);
                            counter = 0;
                        }
                        let info_hash_data: &[u8] = result.get(self.config.db_structure.table_whitelist_info_hash.clone().as_str());
                        let info_hash_decoded = hex::decode(info_hash_data).unwrap();
                        let info_hash = <[u8; 20]>::try_from(info_hash_decoded[0..20].as_ref()).unwrap();
                        return_data_whitelist.push(InfoHash(info_hash));
                        counter += 1;
                        total_whitelist += 1;
                    }

                    info!("[MySQL] Loaded {} whitelists...", total_whitelist);
                    Ok(return_data_whitelist)
                }
                DatabaseDrivers::pgsql => {
                    let pool = &self.pgsql.clone().unwrap().pool;

                    let query = format!(
                        "SELECT {} FROM {}",
                        self.config.db_structure.table_whitelist_info_hash,
                        self.config.db_structure.db_whitelist
                    );
                    let mut rows = sqlx::query(
                        query.as_str()
                    ).fetch(pool);
                    while let Some(result) = rows.try_next().await? {
                        if counter == 100000 {
                            info!("[PgSQL] Loaded {} whitelists...", total_whitelist);
                            counter = 0;
                        }
                        let info_hash_data: &[u8] = result.get(self.config.db_structure.table_whitelist_info_hash.clone().as_str());
                        let info_hash_decoded = hex::decode(info_hash_data).unwrap();
                        let info_hash = <[u8; 20]>::try_from(info_hash_decoded[0..20].as_ref()).unwrap();
                        return_data_whitelist.push(InfoHash(info_hash));
                        counter += 1;
                        total_whitelist += 1;
                    }

                    info!("[PgSQL] Loaded {} whitelists...", total_whitelist);
                    Ok(return_data_whitelist)
                }
            };
        }

        Err(Error::RowNotFound)
    }

    pub async fn load_blacklist(&self) -> Result<Vec<InfoHash>, Error>
    {
        let mut return_data_blacklist = vec![];
        let mut counter = 0u64;
        let mut total_blacklist = 0u64;

        if self.engine.is_some() {
            return match self.engine.clone().unwrap() {
                DatabaseDrivers::sqlite3 => {
                    let pool = &self.sqlite.clone().unwrap().pool;

                    let query = format!(
                        "SELECT {} FROM {}",
                        self.config.db_structure.table_blacklist_info_hash,
                        self.config.db_structure.db_blacklist
                    );
                    let mut rows = sqlx::query(
                        query.as_str()
                    ).fetch(pool);
                    while let Some(result) = rows.try_next().await? {
                        if counter == 100000 {
                            info!("[SQLite3] Loaded {} blacklists...", total_blacklist);
                            counter = 0;
                        }
                        let info_hash_data: &str = result.get(self.config.db_structure.table_blacklist_info_hash.clone().as_str());
                        let info_hash_decoded = hex::decode(info_hash_data).unwrap();
                        let info_hash = <[u8; 20]>::try_from(info_hash_decoded[0..20].as_ref()).unwrap();
                        return_data_blacklist.push(InfoHash(info_hash));
                        counter += 1;
                        total_blacklist += 1;
                    }

                    info!("[SQLite3] Loaded {} blacklists...", total_blacklist);
                    Ok(return_data_blacklist)
                }
                DatabaseDrivers::mysql => {
                    let pool = &self.mysql.clone().unwrap().pool;

                    let query = format!(
                        "SELECT `{}` FROM `{}`",
                        self.config.db_structure.table_blacklist_info_hash,
                        self.config.db_structure.db_blacklist
                    );
                    let mut rows = sqlx::query(
                        query.as_str()
                    ).fetch(pool);
                    while let Some(result) = rows.try_next().await? {
                        if counter == 100000 {
                            info!("[MySQL] Loaded {} blacklists...", total_blacklist);
                            counter = 0;
                        }
                        let info_hash_data: &[u8] = result.get(self.config.db_structure.table_blacklist_info_hash.clone().as_str());
                        let info_hash_decoded = hex::decode(info_hash_data).unwrap();
                        let info_hash = <[u8; 20]>::try_from(info_hash_decoded[0..20].as_ref()).unwrap();
                        return_data_blacklist.push(InfoHash(info_hash));
                        counter += 1;
                        total_blacklist += 1;
                    }

                    info!("[MySQL] Loaded {} blacklists...", total_blacklist);
                    Ok(return_data_blacklist)
                }
                DatabaseDrivers::pgsql => {
                    let pool = &self.pgsql.clone().unwrap().pool;

                    let query = format!(
                        "SELECT {} FROM {}",
                        self.config.db_structure.table_blacklist_info_hash,
                        self.config.db_structure.db_blacklist
                    );
                    let mut rows = sqlx::query(
                        query.as_str()
                    ).fetch(pool);
                    while let Some(result) = rows.try_next().await? {
                        if counter == 100000 {
                            info!("[PgSQL] Loaded {} blacklists...", total_blacklist);
                            counter = 0;
                        }
                        let info_hash_data: &[u8] = result.get(self.config.db_structure.table_blacklist_info_hash.clone().as_str());
                        let info_hash_decoded = hex::decode(info_hash_data).unwrap();
                        let info_hash = <[u8; 20]>::try_from(info_hash_decoded[0..20].as_ref()).unwrap();
                        return_data_blacklist.push(InfoHash(info_hash));
                        counter += 1;
                        total_blacklist += 1;
                    }

                    info!("[PgSQL] Loaded {} blacklists...", total_blacklist);
                    Ok(return_data_blacklist)
                }
            };
        }

        Err(Error::RowNotFound)
    }

    pub async fn load_keys(&self) -> Result<Vec<(InfoHash, i64)>, Error>
    {
        let mut return_data_keys = vec![];
        let mut counter = 0u64;
        let mut total_keys = 0u64;

        if self.engine.is_some() {
            return match self.engine.clone().unwrap() {
                DatabaseDrivers::sqlite3 => {
                    let pool = &self.sqlite.clone().unwrap().pool;

                    let query = format!(
                        "SELECT {},{} FROM {}",
                        self.config.db_structure.table_keys_hash,
                        self.config.db_structure.table_keys_timeout,
                        self.config.db_structure.db_keys
                    );
                    let mut rows = sqlx::query(
                        query.as_str()
                    ).fetch(pool);
                    while let Some(result) = rows.try_next().await? {
                        if counter == 100000 {
                            info!("[SQLite3] Loaded {} keys...", total_keys);
                            counter = 0;
                        }
                        let hash_data: &str = result.get(self.config.db_structure.table_keys_hash.clone().as_str());
                        let hash_decoded = hex::decode(hash_data).unwrap();
                        let timeout_data: i64 = result.get(self.config.db_structure.table_keys_timeout.clone().as_str());
                        let hash = <[u8; 20]>::try_from(hash_decoded[0..20].as_ref()).unwrap();
                        return_data_keys.push((InfoHash(hash), timeout_data));
                        counter += 1;
                        total_keys += 1;
                    }

                    info!("[SQLite3] Loaded {} keys...", total_keys);
                    Ok(return_data_keys)
                }
                DatabaseDrivers::mysql => {
                    let pool = &self.mysql.clone().unwrap().pool;

                    let query = format!(
                        "SELECT `{}`,`{}` FROM `{}`",
                        self.config.db_structure.table_keys_hash,
                        self.config.db_structure.table_keys_timeout,
                        self.config.db_structure.db_keys
                    );
                    let mut rows = sqlx::query(
                        query.as_str()
                    ).fetch(pool);
                    while let Some(result) = rows.try_next().await? {
                        if counter == 100000 {
                            info!("[MySQL] Loaded {} keys...", total_keys);
                            counter = 0;
                        }
                        let hash_data: &[u8] = result.get(self.config.db_structure.table_keys_hash.clone().as_str());
                        let hash_decoded = hex::decode(hash_data).unwrap();
                        let timeout_data: i64 = result.get(self.config.db_structure.table_keys_timeout.clone().as_str());
                        let hash = <[u8; 20]>::try_from(hash_decoded[0..20].as_ref()).unwrap();
                        return_data_keys.push((InfoHash(hash), timeout_data));
                        counter += 1;
                        total_keys += 1;
                    }

                    info!("[MySQL] Loaded {} keys...", total_keys);
                    Ok(return_data_keys)
                }
                DatabaseDrivers::pgsql => {
                    let pool = &self.pgsql.clone().unwrap().pool;

                    let query = format!(
                        "SELECT {},{} FROM {}",
                        self.config.db_structure.table_keys_hash,
                        self.config.db_structure.table_keys_timeout,
                        self.config.db_structure.db_keys
                    );
                    let mut rows = sqlx::query(
                        query.as_str()
                    ).fetch(pool);
                    while let Some(result) = rows.try_next().await? {
                        if counter == 100000 {
                            info!("[PgSQL] Loaded {} keys...", total_keys);
                            counter = 0;
                        }
                        let hash_data: &[u8] = result.get(self.config.db_structure.table_keys_hash.clone().as_str());
                        let hash_decoded = hex::decode(hash_data).unwrap();
                        let timeout_data: i64 = result.get(self.config.db_structure.table_keys_timeout.clone().as_str());
                        let hash = <[u8; 20]>::try_from(hash_decoded[0..20].as_ref()).unwrap();
                        return_data_keys.push((InfoHash(hash), timeout_data));
                        counter += 1;
                        total_keys += 1;
                    }

                    info!("[PgSQL] Loaded {} keys...", total_keys);
                    Ok(return_data_keys)
                }
            };
        }

        Err(Error::RowNotFound)
    }

    pub async fn load_torrents(&self, tracker: Arc<TorrentTracker>) -> Result<(u64, u64), Error>
    {
        let mut counter = 0u64;
        let mut total_torrents = 0u64;
        let mut total_completes = 0u64;

        if self.engine.is_some() {
            return match self.engine.clone().unwrap() {
                DatabaseDrivers::sqlite3 => {
                    let pool = &self.sqlite.clone().unwrap().pool;

                    let query = format!(
                        "SELECT {},{} FROM {}",
                        self.config.db_structure.table_torrents_info_hash,
                        self.config.db_structure.table_torrents_completed,
                        self.config.db_structure.db_torrents
                    );
                    let mut rows = sqlx::query(
                        query.as_str()
                    ).fetch(pool);
                    let mut torrents_parsing = HashMap::new();
                    while let Some(result) = rows.try_next().await? {
                        if counter == 100000 {
                            tracker.add_torrents(torrents_parsing.clone(), false).await;
                            torrents_parsing.clear();
                            info!("[SQLite3] Loaded {} torrents...", total_torrents);
                            counter = 0;
                        }
                        let info_hash_data: &str = result.get(self.config.db_structure.table_torrents_info_hash.clone().as_str());
                        let info_hash_decoded = hex::decode(info_hash_data).unwrap();
                        let completed_data: i64 = result.get(self.config.db_structure.table_torrents_completed.clone().as_str());
                        let info_hash = <[u8; 20]>::try_from(info_hash_decoded[0..20].as_ref()).unwrap();
                        torrents_parsing.insert(InfoHash(info_hash), TorrentEntryItem {
                            completed: completed_data,
                            seeders: 0,
                            leechers: 0,
                        });
                        counter += 1;
                        total_torrents += 1;
                        total_completes += completed_data as u64;
                    }

                    if counter != 0 {
                        tracker.add_torrents(torrents_parsing.clone(), false).await;
                        torrents_parsing.clear();
                    }

                    info!("[SQLite3] Loaded {} torrents...", total_torrents);
                    Ok((total_torrents, total_completes))
                }
                DatabaseDrivers::mysql => {
                    let pool = &self.mysql.clone().unwrap().pool;

                    let query = format!(
                        "SELECT `{}`,`{}` FROM `{}`",
                        self.config.db_structure.table_torrents_info_hash,
                        self.config.db_structure.table_torrents_completed,
                        self.config.db_structure.db_torrents
                    );
                    let mut rows = sqlx::query(
                        query.as_str()
                    ).fetch(pool);
                    let mut torrents_parsing = HashMap::new();
                    while let Some(result) = rows.try_next().await? {
                        if counter == 100000 {
                            tracker.add_torrents(torrents_parsing.clone(), false).await;
                            torrents_parsing.clear();
                            info!("[MySQL] Loaded {} torrents...", total_torrents);
                            counter = 0;
                        }
                        let info_hash_data: &[u8] = result.get(self.config.db_structure.table_torrents_info_hash.clone().as_str());
                        let info_hash_decoded = hex::decode(info_hash_data).unwrap();
                        let completed_data: i64 = result.get(self.config.db_structure.table_torrents_completed.clone().as_str());
                        let info_hash = <[u8; 20]>::try_from(info_hash_decoded[0..20].as_ref()).unwrap();
                        torrents_parsing.insert(InfoHash(info_hash), TorrentEntryItem {
                            completed: completed_data,
                            seeders: 0,
                            leechers: 0,
                        });
                        counter += 1;
                        total_torrents += 1;
                        total_completes += completed_data as u64;
                    }

                    if counter != 0 {
                        tracker.add_torrents(torrents_parsing.clone(), false).await;
                        torrents_parsing.clear();
                    }

                    info!("[MySQL] Loaded {} torrents...", total_torrents);
                    Ok((total_torrents, total_completes))
                }
                DatabaseDrivers::pgsql => {
                    let pool = &self.pgsql.clone().unwrap().pool;

                    let query = format!(
                        "SELECT {},{} FROM {}",
                        self.config.db_structure.table_torrents_info_hash,
                        self.config.db_structure.table_torrents_completed,
                        self.config.db_structure.db_torrents
                    );
                    let mut rows = sqlx::query(
                        query.as_str()
                    ).fetch(pool);
                    let mut torrents_parsing = HashMap::new();
                    while let Some(result) = rows.try_next().await? {
                        if counter == 100000 {
                            tracker.add_torrents(torrents_parsing.clone(), false).await;
                            torrents_parsing.clear();
                            info!("[PgSQL] Loaded {} torrents...", total_torrents);
                            counter = 0;
                        }
                        let info_hash_data: &[u8] = result.get(self.config.db_structure.table_torrents_info_hash.clone().as_str());
                        let info_hash_decoded = hex::decode(info_hash_data).unwrap();
                        let completed_data: i64 = result.get(self.config.db_structure.table_torrents_completed.clone().as_str());
                        let info_hash = <[u8; 20]>::try_from(info_hash_decoded[0..20].as_ref()).unwrap();
                        torrents_parsing.insert(InfoHash(info_hash), TorrentEntryItem {
                            completed: completed_data,
                            seeders: 0,
                            leechers: 0,
                        });
                        counter += 1;
                        total_torrents += 1;
                        total_completes += completed_data as u64;
                    }

                    if counter != 0 {
                        tracker.add_torrents(torrents_parsing.clone(), false).await;
                        torrents_parsing.clear();
                    }

                    info!("[PgSQL] Loaded {} torrents...", total_torrents);
                    Ok((total_torrents, total_completes))
                }
            };
        }

        Err(Error::RowNotFound)
    }

    pub async fn save_whitelist(&self, whitelists: HashMap<InfoHash, i64>) -> Result<(), Error>
    {
        if self.engine.is_some() {
            return match self.engine.clone().unwrap() {
                DatabaseDrivers::sqlite3 => {
                    let pool = &self.sqlite.clone().unwrap().pool;

                    let mut whitelist_transaction = pool.begin().await?;
                    let mut whitelist_handled_entries = 0u64;
                    for (info_hash, value) in whitelists.iter() {
                        if value == &2 {
                            whitelist_handled_entries += 1;
                            match sqlx::query(&format!(
                                "INSERT OR IGNORE INTO {} ({}) VALUES ('{}')",
                                self.config.db_structure.db_whitelist,
                                self.config.db_structure.table_whitelist_info_hash,
                                info_hash
                            ))
                                .execute(&mut whitelist_transaction)
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
                                self.config.db_structure.db_whitelist,
                                self.config.db_structure.table_whitelist_info_hash,
                                info_hash
                            ))
                                .execute(&mut whitelist_transaction)
                                .await {
                                Ok(_) => {}
                                Err(e) => {
                                    error!("[SQLite3] Error: {}", e.to_string());
                                    return Err(e);
                                }
                            }
                        }
                    }
                    match whitelist_transaction.commit().await {
                        Ok(_) => {}
                        Err(e) => {
                            error!("[SQLite3] Error: {}", e.to_string());
                            return Err(e);
                        }
                    };

                    Ok(())
                }
                DatabaseDrivers::mysql => {
                    let pool = &self.mysql.clone().unwrap().pool;

                    let mut whitelist_transaction = pool.begin().await?;
                    let mut whitelist_handled_entries = 0u64;
                    // let _ = sqlx::query(&*format!("TRUNCATE TABLE {}", self.config.db_structure.db_whitelist)).execute(&mut whitelist_transaction).await?;
                    for (info_hash, value) in whitelists.iter() {
                        if value == &2 {
                            whitelist_handled_entries += 1;
                            match sqlx::query(&format!(
                                "INSERT IGNORE INTO {} ({}) VALUES ('{}')",
                                self.config.db_structure.db_whitelist,
                                self.config.db_structure.table_whitelist_info_hash,
                                info_hash
                            ))
                                .execute(&mut whitelist_transaction)
                                .await {
                                Ok(_) => {}
                                Err(e) => {
                                    error!("[MySQL] Error: {}", e.to_string());
                                    return Err(e);
                                }
                            }

                            if (whitelist_handled_entries as f64 / 1000f64).fract() == 0.0 || whitelists.len() as u64 == whitelist_handled_entries {
                                info!("[MySQL] Handled {} whitelists", whitelist_handled_entries);
                            }
                        }
                        if value == &0 {
                            match sqlx::query(&format!(
                                "DELETE FROM {} WHERE {} = '{}'",
                                self.config.db_structure.db_whitelist,
                                self.config.db_structure.table_whitelist_info_hash,
                                info_hash
                            ))
                                .execute(&mut whitelist_transaction)
                                .await {
                                Ok(_) => {}
                                Err(e) => {
                                    error!("[MySQL] Error: {}", e.to_string());
                                    return Err(e);
                                }
                            }
                        }
                    }
                    match whitelist_transaction.commit().await {
                        Ok(_) => {}
                        Err(e) => {
                            error!("[MySQL] Error: {}", e.to_string());
                            return Err(e);
                        }
                    };

                    Ok(())
                }
                DatabaseDrivers::pgsql => {
                    let pool = &self.pgsql.clone().unwrap().pool;

                    let mut whitelist_transaction = pool.begin().await?;
                    let mut whitelist_handled_entries = 0u64;
                    match sqlx::query(&format!("TRUNCATE TABLE {} RESTART IDENTITY", self.config.db_structure.db_whitelist)).execute(&mut whitelist_transaction).await {
                        Ok(_) => {}
                        Err(e) => {
                            error!("[PgSQL] Error: {}", e.to_string());
                            return Err(e);
                        }
                    }
                    for (info_hash, value) in whitelists.iter() {
                        if value == &2 {
                            whitelist_handled_entries += 1;
                            match sqlx::query(&format!(
                                "INSERT INTO {} ({}) VALUES ('{}') ON CONFLICT ({}) DO NOTHING;",
                                self.config.db_structure.db_whitelist,
                                self.config.db_structure.table_whitelist_info_hash,
                                info_hash,
                                self.config.db_structure.table_whitelist_info_hash
                            ))
                                .execute(&mut whitelist_transaction)
                                .await {
                                Ok(_) => {}
                                Err(e) => {
                                    error!("[PgSQL] Error: {}", e.to_string());
                                    return Err(e);
                                }
                            }

                            if (whitelist_handled_entries as f64 / 1000f64).fract() == 0.0 || whitelists.len() as u64 == whitelist_handled_entries {
                                info!("[PgSQL] Handled {} whitelists", whitelist_handled_entries);
                            }
                        }
                        if value == &0 {
                            match sqlx::query(&format!(
                                "DELETE FROM {} WHERE {} = '{}';",
                                self.config.db_structure.db_whitelist,
                                self.config.db_structure.table_whitelist_info_hash,
                                info_hash
                            ))
                                .execute(&mut whitelist_transaction)
                                .await {
                                Ok(_) => {}
                                Err(e) => {
                                    error!("[PgSQL] Error: {}", e.to_string());
                                    return Err(e);
                                }
                            }
                        }
                    }
                    match whitelist_transaction.commit().await {
                        Ok(_) => {}
                        Err(e) => {
                            error!("[PgSQL] Error: {}", e.to_string());
                            return Err(e);
                        }
                    };

                    Ok(())
                }
            };
        }

        Err(Error::RowNotFound)
    }

    pub async fn save_blacklist(&self, blacklists: Vec<InfoHash>) -> Result<(), Error>
    {
        if self.engine.is_some() {
            return match self.engine.clone().unwrap() {
                DatabaseDrivers::sqlite3 => {
                    let pool = &self.sqlite.clone().unwrap().pool;

                    let mut blacklist_transaction = pool.begin().await?;
                    let mut blacklist_handled_entries = 0u64;
                    for info_hash in blacklists.iter() {
                        blacklist_handled_entries += 1;
                        match sqlx::query(&format!(
                            "INSERT OR REPLACE INTO {} ({}) VALUES ('{}')",
                            self.config.db_structure.db_blacklist,
                            self.config.db_structure.table_blacklist_info_hash,
                            info_hash
                        ))
                            .execute(&mut blacklist_transaction)
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
                    match blacklist_transaction.commit().await {
                        Ok(_) => {}
                        Err(e) => {
                            error!("[SQLite3] Error: {}", e.to_string());
                            return Err(e);
                        }
                    };

                    Ok(())
                }
                DatabaseDrivers::mysql => {
                    let pool = &self.mysql.clone().unwrap().pool;

                    let mut blacklist_transaction = pool.begin().await?;
                    let mut blacklist_handled_entries = 0u64;
                    let _ = sqlx::query(&format!("TRUNCATE TABLE {}", self.config.db_structure.db_blacklist)).execute(&mut blacklist_transaction).await?;
                    for info_hash in blacklists.iter() {
                        blacklist_handled_entries += 1;
                        match sqlx::query(&format!(
                            "INSERT INTO {} ({}) VALUES ('{}')",
                            self.config.db_structure.db_blacklist,
                            self.config.db_structure.table_blacklist_info_hash,
                            info_hash
                        ))
                            .execute(&mut blacklist_transaction)
                            .await {
                            Ok(_) => {}
                            Err(e) => {
                                error!("[MySQL] Error: {}", e.to_string());
                                return Err(e);
                            }
                        }

                        if (blacklist_handled_entries as f64 / 1000f64).fract() == 0.0 || blacklists.len() as u64 == blacklist_handled_entries {
                            info!("[MySQL] Handled {} blacklists", blacklist_handled_entries);
                        }
                    }
                    match blacklist_transaction.commit().await {
                        Ok(_) => {}
                        Err(e) => {
                            error!("[MySQL] Error: {}", e.to_string());
                            return Err(e);
                        }
                    };

                    Ok(())
                }
                DatabaseDrivers::pgsql => {
                    let pool = &self.pgsql.clone().unwrap().pool;

                    let mut blacklist_transaction = pool.begin().await?;
                    let mut blacklist_handled_entries = 0u64;
                    let _ = sqlx::query(&format!("TRUNCATE TABLE {} RESTART IDENTITY", self.config.db_structure.db_blacklist)).execute(&mut blacklist_transaction).await?;
                    for info_hash in blacklists.iter() {
                        blacklist_handled_entries += 1;
                        match sqlx::query(&format!(
                            "INSERT INTO {} ({}) VALUES ('{}')",
                            self.config.db_structure.db_blacklist,
                            self.config.db_structure.table_blacklist_info_hash,
                            info_hash
                        ))
                            .execute(&mut blacklist_transaction)
                            .await {
                            Ok(_) => {}
                            Err(e) => {
                                error!("[PgSQL] Error: {}", e.to_string());
                                return Err(e);
                            }
                        }

                        if (blacklist_handled_entries as f64 / 1000f64).fract() == 0.0 || blacklists.len() as u64 == blacklist_handled_entries {
                            info!("[PgSQL] Handled {} blacklists", blacklist_handled_entries);
                        }
                    }
                    match blacklist_transaction.commit().await {
                        Ok(_) => {}
                        Err(e) => {
                            error!("[PgSQL] Error: {}", e.to_string());
                            return Err(e);
                        }
                    };

                    Ok(())
                }
            };
        }

        Err(Error::RowNotFound)
    }

    pub async fn save_keys(&self, keys: Vec<(InfoHash, i64)>) -> Result<(), Error>
    {
        if self.engine.is_some() {
            return match self.engine.clone().unwrap() {
                DatabaseDrivers::sqlite3 => {
                    let pool = &self.sqlite.clone().unwrap().pool;

                    let mut keys_transaction = pool.begin().await?;
                    let mut keys_handled_entries = 0u64;
                    for (hash, timeout) in keys.iter() {
                        keys_handled_entries += 1;
                        match sqlx::query(&format!(
                            "INSERT OR REPLACE INTO {} ({},{}) VALUES ('{}',{})",
                            self.config.db_structure.db_keys,
                            self.config.db_structure.table_keys_hash,
                            self.config.db_structure.table_keys_timeout,
                            hash,
                            timeout.clone()
                        ))
                            .execute(&mut keys_transaction)
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
                    match keys_transaction.commit().await {
                        Ok(_) => {}
                        Err(e) => {
                            error!("[SQLite3] Error: {}", e.to_string());
                            return Err(e);
                        }
                    };

                    Ok(())
                }
                DatabaseDrivers::mysql => {
                    let pool = &self.mysql.clone().unwrap().pool;

                    let mut keys_transaction = pool.begin().await?;
                    let mut keys_handled_entries = 0u64;
                    for (hash, timeout) in keys.iter() {
                        keys_handled_entries += 1;
                        match sqlx::query(&format!(
                            "INSERT INTO {} (`{}`,`{}`) VALUES ('{}',{}) ON DUPLICATE KEY UPDATE `{}`=VALUES(`{}`)",
                            self.config.db_structure.db_keys,
                            self.config.db_structure.table_keys_hash,
                            self.config.db_structure.table_keys_timeout,
                            hash,
                            timeout.clone(),
                            self.config.db_structure.table_keys_timeout,
                            self.config.db_structure.table_keys_timeout
                        ))
                            .execute(&mut keys_transaction)
                            .await {
                            Ok(_) => {}
                            Err(e) => {
                                error!("[MySQL] Error: {}", e.to_string());
                                return Err(e);
                            }
                        }

                        if (keys_handled_entries as f64 / 1000f64).fract() == 0.0 || keys.len() as u64 == keys_handled_entries {
                            info!("[MySQL] Handled {} keys", keys_handled_entries);
                        }
                    }
                    match keys_transaction.commit().await {
                        Ok(_) => {}
                        Err(e) => {
                            error!("[MySQL] Error: {}", e.to_string());
                            return Err(e);
                        }
                    };

                    Ok(())
                }
                DatabaseDrivers::pgsql => {
                    let pool = &self.pgsql.clone().unwrap().pool;

                    let mut keys_transaction = pool.begin().await?;
                    let mut keys_handled_entries = 0u64;
                    for (hash, timeout) in keys.iter() {
                        keys_handled_entries += 1;
                        match sqlx::query(&format!(
                            "INSERT INTO {} ({},{}) VALUES ('{}',{}) ON CONFLICT ({}) DO UPDATE SET {}=excluded.{}",
                            self.config.db_structure.db_keys,
                            self.config.db_structure.table_keys_hash,
                            self.config.db_structure.table_keys_timeout,
                            hash,
                            timeout.clone(),
                            self.config.db_structure.table_keys_hash,
                            self.config.db_structure.table_keys_timeout,
                            self.config.db_structure.table_keys_timeout
                        ))
                            .execute(&mut keys_transaction)
                            .await {
                            Ok(_) => {}
                            Err(e) => {
                                error!("[PgSQL] Error: {}", e.to_string());
                                return Err(e);
                            }
                        }

                        if (keys_handled_entries as f64 / 1000f64).fract() == 0.0 || keys.len() as u64 == keys_handled_entries {
                            info!("[PgSQL] Handled {} keys", keys_handled_entries);
                        }
                    }
                    match keys_transaction.commit().await {
                        Ok(_) => {}
                        Err(e) => {
                            error!("[PgSQL] Error: {}", e.to_string());
                            return Err(e);
                        }
                    };

                    Ok(())
                }
            };
        }

        Err(Error::RowNotFound)
    }

    pub async fn save_torrents(&self, torrents: HashMap<InfoHash, i64>) -> Result<(), Error>
    {
        if self.engine.is_some() {
            return match self.engine.clone().unwrap() {
                DatabaseDrivers::sqlite3 => {
                    let pool = &self.sqlite.clone().unwrap().pool;
                    let mut torrents_transaction = pool.begin().await?;
                    let mut torrents_handled_entries = 0u64;
                    for (info_hash, completed) in torrents.iter() {
                        torrents_handled_entries += 1;
                        match sqlx::query(&format!(
                            "INSERT OR REPLACE INTO {} ({},{}) VALUES ('{}',{})",
                            self.config.db_structure.db_torrents,
                            self.config.db_structure.table_torrents_info_hash,
                            self.config.db_structure.table_torrents_completed,
                            info_hash,
                            completed.clone()
                        ))
                            .execute(&mut torrents_transaction)
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
                            torrents_transaction = pool.begin().await?
                        }
                    }
                    match torrents_transaction.commit().await {
                        Ok(_) => {}
                        Err(e) => {
                            error!("[SQLite3] Error: {}", e.to_string());
                            return Err(e);
                        }
                    };

                    Ok(())
                }
                DatabaseDrivers::mysql => {
                    let pool = &self.mysql.clone().unwrap().pool;

                    let mut torrents_transaction = pool.begin().await?;
                    let mut torrents_handled_entries = 0u64;
                    for (info_hash, completed) in torrents.iter() {
                        torrents_handled_entries += 1;
                        match sqlx::query(&format!(
                            "INSERT INTO {} (`{}`,`{}`) VALUES ('{}',{}) ON DUPLICATE KEY UPDATE `{}`=VALUES(`{}`)",
                            self.config.db_structure.db_torrents,
                            self.config.db_structure.table_torrents_info_hash,
                            self.config.db_structure.table_torrents_completed,
                            info_hash,
                            completed.clone(),
                            self.config.db_structure.table_torrents_completed,
                            self.config.db_structure.table_torrents_completed
                        ))
                            .execute(&mut torrents_transaction)
                            .await {
                            Ok(_) => {}
                            Err(e) => {
                                error!("[MySQL] Error: {}", e.to_string());
                                return Err(e);
                            }
                        }

                        if (torrents_handled_entries as f64 / 1000f64).fract() == 0.0 || torrents.len() as u64 == torrents_handled_entries {
                            info!("[MySQL] Handled {} torrents", torrents_handled_entries);
                        }
                    }
                    match torrents_transaction.commit().await {
                        Ok(_) => {}
                        Err(e) => {
                            error!("[MySQL] Error: {}", e.to_string());
                            return Err(e);
                        }
                    };

                    Ok(())
                }
                DatabaseDrivers::pgsql => {
                    let pool = &self.pgsql.clone().unwrap().pool;

                    let mut torrents_transaction = pool.begin().await?;
                    let mut torrents_handled_entries = 0u64;
                    for (info_hash, completed) in torrents.iter() {
                        torrents_handled_entries += 1;
                        match sqlx::query(&format!(
                            "INSERT INTO {} ({},{}) VALUES ('{}',{}) ON CONFLICT ({}) DO UPDATE SET {}=excluded.{}",
                            self.config.db_structure.db_torrents,
                            self.config.db_structure.table_torrents_info_hash,
                            self.config.db_structure.table_torrents_completed,
                            info_hash,
                            completed.clone(),
                            self.config.db_structure.table_torrents_info_hash,
                            self.config.db_structure.table_torrents_completed,
                            self.config.db_structure.table_torrents_completed
                        ))
                            .execute(&mut torrents_transaction)
                            .await {
                            Ok(_) => {}
                            Err(e) => {
                                error!("[PgSQL] Error: {}", e.to_string());
                                return Err(e);
                            }
                        }

                        if (torrents_handled_entries as f64 / 1000f64).fract() == 0.0 || torrents.len() as u64 == torrents_handled_entries {
                            info!("[PgSQL] Handled {} torrents", torrents_handled_entries);
                        }
                    }
                    match torrents_transaction.commit().await {
                        Ok(_) => {}
                        Err(e) => {
                            error!("[PgSQL] Error: {}", e.to_string());
                            return Err(e);
                        }
                    };

                    Ok(())
                }
            };
        }

        Err(Error::RowNotFound)
    }
}