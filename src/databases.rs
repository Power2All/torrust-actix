use std::collections::HashMap;
use std::process::exit;
use std::str::FromStr;
use std::time::Duration;
use futures::TryStreamExt;
use log::{info, error};
use scc::ebr::Arc;
use sqlx::mysql::{MySqlConnectOptions, MySqlPoolOptions};
use sqlx::{Error, MySql, Pool, Postgres, Row, Sqlite, ConnectOptions};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool};
use serde::{Deserialize, Serialize};
use crate::common::InfoHash;
use crate::config::Configuration;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DatabaseDrivers {
    SQLite3,
    MySQL,
    PgSQL
}

#[derive(Clone)]
pub struct DatabaseConnectorMySQL {
    pool: Pool<MySql>
}

#[derive(Clone)]
pub struct DatabaseConnectorSQLite {
    pool: Pool<Sqlite>
}

#[derive(Clone)]
pub struct DatabaseConnectorPgSQL {
    pool: Pool<Postgres>
}

#[derive(Clone)]
pub struct DatabaseConnector {
    config: Arc<Configuration>,
    mysql: Option<DatabaseConnectorMySQL>,
    sqlite: Option<DatabaseConnectorSQLite>,
    pgsql: Option<DatabaseConnectorPgSQL>,
    engine: Option<DatabaseDrivers>
}

impl DatabaseConnectorSQLite {
    pub async fn create(dsl: &str) -> Result<Pool<Sqlite>, Error>
    {
        let options = SqliteConnectOptions::from_str(dsl)?;
        options
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .read_only(false)
            .log_statements(log::LevelFilter::Debug)
            .log_slow_statements(log::LevelFilter::Debug, Duration::from_secs(1));
        SqlitePool::connect(dsl).await
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

        let mut structure = DatabaseConnector{
            config: config.clone(),
            mysql: None,
            sqlite: None,
            pgsql: None,
            engine: None
        };

        match &config.db_driver {
            DatabaseDrivers::SQLite3 => {
                let sqlite_connect = DatabaseConnectorSQLite::create(&config.db_path).await;
                if sqlite_connect.is_err() {
                    error!("[SQLite] Unable to open the database {}", &config.db_path);
                    error!("[SQLite] Message: {:#?}", sqlite_connect.unwrap_err().into_database_error());
                    exit(1);
                }
                structure.sqlite = Some(DatabaseConnectorSQLite {
                    pool: sqlite_connect.unwrap()
                });
                structure.engine = Some(DatabaseDrivers::SQLite3);
                let pool = &structure.sqlite.clone().unwrap().pool;
                let _ = sqlx::query(
                    format!(
                        "CREATE TABLE IF NOT EXISTS {} ({} VARCHAR(40) NOT NULL UNIQUE, {} INTEGER DEFAULT 0 NOT NULL)",
                        config.db_structure.db_torrents,
                        config.db_structure.table_torrents_info_hash,
                        config.db_structure.table_torrents_completed
                    ).as_str()
                ).execute(pool).await;
                let _ = sqlx::query(
                    format!(
                        "CREATE TABLE IF NOT EXISTS {} ({} VARCHAR(40) NOT NULL UNIQUE)",
                        config.db_structure.db_whitelist,
                        config.db_structure.table_whitelist_info_hash
                    ).as_str()
                ).execute(pool).await;
                let _ = sqlx::query(
                    format!(
                        "CREATE TABLE IF NOT EXISTS {} ({} VARCHAR(40) NOT NULL UNIQUE)",
                        config.db_structure.db_blacklist,
                        config.db_structure.table_blacklist_info_hash
                    ).as_str()
                ).execute(pool).await;
            }
            DatabaseDrivers::MySQL => {
                let mysql_connect = DatabaseConnectorMySQL::create(&config.db_path).await;
                if mysql_connect.is_err() {
                    error!("[MySQL] Unable to connect to MySQL on DSL {}", &config.db_path);
                    exit(1);
                }
                structure.mysql = Some(DatabaseConnectorMySQL {
                    pool: mysql_connect.unwrap()
                });
                structure.engine = Some(DatabaseDrivers::MySQL);
            }
            DatabaseDrivers::PgSQL => {
                let pgsql_connect = DatabaseConnectorPgSQL::create(&config.db_path).await;
                if pgsql_connect.is_err() {
                    error!("[PgSQL] Unable to connect to PostgreSQL on DSL {}", &config.db_path)
                }
                structure.pgsql = Some(DatabaseConnectorPgSQL {
                    pool: pgsql_connect.unwrap()
                });
                structure.engine = Some(DatabaseDrivers::PgSQL);
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
            match self.engine.clone().unwrap() {
                DatabaseDrivers::SQLite3 => {
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
                        if counter == 10000 {
                            info!("[SQLite3] Loaded {} whitelists...", total_whitelist);
                            counter = 0;
                        }
                        let infohash_data: &str = result.get(self.config.db_structure.table_whitelist_info_hash.clone().as_str());
                        let infohash_decoded = hex::decode(infohash_data).unwrap();
                        let infohash = <[u8; 20]>::try_from(infohash_decoded[0 .. 20].as_ref()).unwrap();
                        return_data_whitelist.push(InfoHash(infohash));
                        counter += 1;
                        total_whitelist += 1;
                    }

                    info!("[SQLite3] Loaded {} whitelists...", total_whitelist);
                    return Ok(return_data_whitelist);
                }
                DatabaseDrivers::MySQL => {
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
                        if counter == 10000 {
                            info!("[MySQL] Loaded {} whitelists...", total_whitelist);
                            counter = 0;
                        }
                        let infohash_data: &[u8] = result.get(self.config.db_structure.table_whitelist_info_hash.clone().as_str());
                        let infohash = <[u8; 20]>::try_from(infohash_data[0 .. 20].as_ref()).unwrap();
                        return_data_whitelist.push(InfoHash(infohash));
                        counter += 1;
                        total_whitelist += 1;
                    }

                    info!("[MySQL] Loaded {} whitelists...", total_whitelist);
                    return Ok(return_data_whitelist);
                }
                DatabaseDrivers::PgSQL => {
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
                        if counter == 10000 {
                            info!("[PgSQL] Loaded {} whitelists...", total_whitelist);
                            counter = 0;
                        }
                        let infohash_data: &[u8] = result.get(self.config.db_structure.table_whitelist_info_hash.clone().as_str());
                        let infohash = <[u8; 20]>::try_from(infohash_data[0 .. 20].as_ref()).unwrap();
                        return_data_whitelist.push(InfoHash(infohash));
                        counter += 1;
                        total_whitelist += 1;
                    }

                    info!("[PgSQL] Loaded {} whitelists...", total_whitelist);
                    return Ok(return_data_whitelist);
                }
            }
        }

        Err(Error::RowNotFound)
    }

    pub async fn load_blacklist(&self) -> Result<Vec<InfoHash>, Error>
    {
        let mut return_data_blacklist = vec![];
        let mut counter = 0u64;
        let mut total_blacklist = 0u64;

        if self.engine.is_some() {
            match self.engine.clone().unwrap() {
                DatabaseDrivers::SQLite3 => {
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
                        if counter == 10000 {
                            info!("[SQLite3] Loaded {} blacklists...", total_blacklist);
                            counter = 0;
                        }
                        let infohash_data: &str = result.get(self.config.db_structure.table_blacklist_info_hash.clone().as_str());
                        let infohash_decoded = hex::decode(infohash_data).unwrap();
                        let infohash = <[u8; 20]>::try_from(infohash_decoded[0..20].as_ref()).unwrap();
                        return_data_blacklist.push(InfoHash(infohash));
                        counter += 1;
                        total_blacklist += 1;
                    }

                    info!("[SQLite3] Loaded {} blacklists...", total_blacklist);
                    return Ok(return_data_blacklist);
                }
                DatabaseDrivers::MySQL => {
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
                        if counter == 10000 {
                            info!("[MySQL] Loaded {} blacklists...", total_blacklist);
                            counter = 0;
                        }
                        let infohash_data: &[u8] = result.get(self.config.db_structure.table_blacklist_info_hash.clone().as_str());
                        let infohash = <[u8; 20]>::try_from(infohash_data[0..20].as_ref()).unwrap();
                        return_data_blacklist.push(InfoHash(infohash));
                        counter += 1;
                        total_blacklist += 1;
                    }

                    info!("[MySQL] Loaded {} blacklists...", total_blacklist);
                    return Ok(return_data_blacklist);
                }
                DatabaseDrivers::PgSQL => {
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
                        if counter == 10000 {
                            info!("[PgSQL] Loaded {} blacklists...", total_blacklist);
                            counter = 0;
                        }
                        let infohash_data: &[u8] = result.get(self.config.db_structure.table_blacklist_info_hash.clone().as_str());
                        let infohash = <[u8; 20]>::try_from(infohash_data[0..20].as_ref()).unwrap();
                        return_data_blacklist.push(InfoHash(infohash));
                        counter += 1;
                        total_blacklist += 1;
                    }

                    info!("[PgSQL] Loaded {} blacklists...", total_blacklist);
                    return Ok(return_data_blacklist);
                }
            }
        }

        Err(Error::RowNotFound)
    }

    pub async fn load_torrents(&self) -> Result<Vec<(InfoHash, i64)>, Error>
    {
        let mut return_data_torrents = vec![];
        let mut counter = 0u64;
        let mut total_torrents = 0u64;

        if self.engine.is_some() {
            match self.engine.clone().unwrap() {
                DatabaseDrivers::SQLite3 => {
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
                    while let Some(result) = rows.try_next().await? {
                        if counter == 10000 {
                            info!("[SQLite3] Loaded {} torrents...", total_torrents);
                            counter = 0;
                        }
                        let infohash_data: &str = result.get(self.config.db_structure.table_torrents_info_hash.clone().as_str());
                        let infohash_decoded = hex::decode(infohash_data).unwrap();
                        let completed_data: i64 = result.get(self.config.db_structure.table_torrents_completed.clone().as_str());
                        let infohash = <[u8; 20]>::try_from(infohash_decoded[0 .. 20].as_ref()).unwrap();
                        return_data_torrents.push((InfoHash(infohash), completed_data));
                        counter += 1;
                        total_torrents += 1;
                    }

                    info!("[SQLite3] Loaded {} torrents...", total_torrents);
                    return Ok(return_data_torrents);
                }
                DatabaseDrivers::MySQL => {
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
                    while let Some(result) = rows.try_next().await? {
                        if counter == 10000 {
                            info!("[MySQL] Loaded {} torrents...", total_torrents);
                            counter = 0;
                        }
                        let infohash_data: &[u8] = result.get(self.config.db_structure.table_torrents_info_hash.clone().as_str());
                        let completed_data: i64 = result.get(self.config.db_structure.table_torrents_completed.clone().as_str());
                        let infohash = <[u8; 20]>::try_from(infohash_data[0 .. 20].as_ref()).unwrap();
                        return_data_torrents.push((InfoHash(infohash), completed_data));
                        counter += 1;
                        total_torrents += 1;
                    }

                    info!("[MySQL] Loaded {} torrents...", total_torrents);
                    return Ok(return_data_torrents);
                }
                DatabaseDrivers::PgSQL => {
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
                    while let Some(result) = rows.try_next().await? {
                        if counter == 10000 {
                            info!("[PgSQL] Loaded {} torrents...", total_torrents);
                            counter = 0;
                        }
                        let infohash_data: &[u8] = result.get(self.config.db_structure.table_torrents_info_hash.clone().as_str());
                        let completed_data: i64 = result.get(self.config.db_structure.table_torrents_completed.clone().as_str());
                        let infohash = <[u8; 20]>::try_from(infohash_data[0 .. 20].as_ref()).unwrap();
                        return_data_torrents.push((InfoHash(infohash), completed_data));
                        counter += 1;
                        total_torrents += 1;
                    }

                    info!("[PgSQL] Loaded {} torrents...", total_torrents);
                    return Ok(return_data_torrents);
                }
            }
        }

        Err(Error::RowNotFound)
    }

    pub async fn save_whitelist(&self, whitelists: Vec<InfoHash>) -> Result<(), Error>
    {
        if self.engine.is_some() {
            return match self.engine.clone().unwrap() {
                DatabaseDrivers::SQLite3 => {
                    let pool = &self.sqlite.clone().unwrap().pool;

                    let mut whitelists_transaction = pool.begin().await?;
                    let mut whitelists_handled_entries = 0u64;
                    let mut whitelists_insert_entries = Vec::new();
                    let query = format!("DELETE FROM {}", self.config.db_structure.db_whitelist);
                    sqlx::query(&query).execute(&mut whitelists_transaction).await?;
                    for info_hash in whitelists.iter() {
                        whitelists_handled_entries += 1;
                        whitelists_insert_entries.push(format!("('{}')", info_hash).to_string());
                        if whitelists_insert_entries.len() == 10000 {
                            let pre_query = format!(
                                "INSERT OR REPLACE INTO {} ({}) VALUES",
                                self.config.db_structure.db_whitelist,
                                self.config.db_structure.table_whitelist_info_hash
                            );
                            let query = format!(
                                "{} {}",
                                pre_query,
                                whitelists_insert_entries.join(",")
                            );
                            sqlx::query(&query).execute(&mut whitelists_transaction).await?;
                            info!("[SQLite3] Handled {} whitelists", whitelists_handled_entries);
                            whitelists_insert_entries = vec![];
                        }
                    }
                    if !whitelists_insert_entries.is_empty() {
                        let pre_query = format!(
                            "INSERT OR REPLACE INTO {} ({}) VALUES",
                            self.config.db_structure.db_whitelist,
                            self.config.db_structure.table_whitelist_info_hash
                        );
                        let query = format!(
                            "{} {}",
                            pre_query,
                            whitelists_insert_entries.join(",")
                        );
                        sqlx::query(&query).execute(&mut whitelists_transaction).await?;
                        info!("[SQLite3] Handled {} whitelists", whitelists_handled_entries);
                    }
                    match whitelists_transaction.commit().await {
                        Ok(_) => {}
                        Err(e) => {
                            error!("[SQLite3] Error: {}", e.to_string());
                            return Err(e);
                        }
                    };

                    Ok(())
                }
                DatabaseDrivers::MySQL => {
                    let pool = &self.mysql.clone().unwrap().pool;

                    let mut whitelists_transaction = pool.begin().await?;
                    let mut whitelists_handled_entries = 0u64;
                    let mut whitelists_insert_entries = Vec::new();
                    let query = format!("TRUNCATE TABLE {}", self.config.db_structure.db_whitelist);
                    sqlx::query(&query).execute(&mut whitelists_transaction).await?;
                    for info_hash in whitelists.iter() {
                        whitelists_handled_entries += 1;
                        whitelists_insert_entries.push(format!("(UNHEX(\"{}\"))", info_hash).to_string());
                        if whitelists_insert_entries.len() == 10000 {
                            let query = format!(
                                "INSERT INTO {} (`{}`) VALUES {}",
                                self.config.db_structure.db_whitelist,
                                self.config.db_structure.table_whitelist_info_hash,
                                whitelists_insert_entries.join(",")
                            );
                            sqlx::query(&query).execute(&mut whitelists_transaction).await?;
                            info!("[MySQL] Handled {} whitelists", whitelists_handled_entries);
                            whitelists_insert_entries = vec![];
                        }
                    }
                    if !whitelists_insert_entries.is_empty() {
                        let query = format!(
                            "INSERT INTO {} (`{}`) VALUES {}",
                            self.config.db_structure.db_whitelist,
                            self.config.db_structure.table_whitelist_info_hash,
                            whitelists_insert_entries.join(",")
                        );
                        sqlx::query(&query).execute(&mut whitelists_transaction).await?;
                        info!("[MySQL] Handled {} whitelists", whitelists_handled_entries);
                    }
                    match whitelists_transaction.commit().await {
                        Ok(_) => {}
                        Err(e) => {
                            error!("[MySQL] Error: {}", e.to_string());
                            return Err(e);
                        }
                    };

                    Ok(())
                }
                DatabaseDrivers::PgSQL => {
                    let pool = &self.pgsql.clone().unwrap().pool;

                    let mut whitelists_transaction = pool.begin().await?;
                    let mut whitelists_handled_entries = 0u64;
                    let mut whitelists_insert_entries = Vec::new();
                    let query = format!("TRUNCATE TABLE {} RESTART IDENTITY", self.config.db_structure.db_whitelist);
                    sqlx::query(&query).execute(&mut whitelists_transaction).await?;
                    for info_hash in whitelists.iter() {
                        whitelists_handled_entries += 1;
                        whitelists_insert_entries.push(format!("(decode('{}', 'hex'))", info_hash).to_string());
                        if whitelists_insert_entries.len() == 10000 {
                            let query = format!(
                                "INSERT INTO {} ({}) VALUES {}",
                                self.config.db_structure.db_whitelist,
                                self.config.db_structure.table_whitelist_info_hash,
                                whitelists_insert_entries.join(",")
                            );
                            sqlx::query(&query).execute(&mut whitelists_transaction).await?;
                            info!("[PgSQL] Handled {} whitelists", whitelists_handled_entries);
                            whitelists_insert_entries = vec![];
                        }
                    }
                    if !whitelists_insert_entries.is_empty() {
                        let query = format!(
                            "INSERT INTO {} ({}) VALUES {}",
                            self.config.db_structure.db_whitelist,
                            self.config.db_structure.table_whitelist_info_hash,
                            whitelists_insert_entries.join(",")
                        );
                        sqlx::query(&query).execute(&mut whitelists_transaction).await?;
                        info!("[PgSQL] Handled {} whitelists", whitelists_handled_entries);
                    }
                    match whitelists_transaction.commit().await {
                        Ok(_) => {}
                        Err(e) => {
                            error!("[PgSQL] Error: {}", e.to_string());
                            return Err(e);
                        }
                    };

                    Ok(())
                }
            }
        }

        Err(Error::RowNotFound)
    }

    pub async fn save_blacklist(&self, blacklists: Vec<InfoHash>) -> Result<(), Error>
    {
        if self.engine.is_some() {
            return match self.engine.clone().unwrap() {
                DatabaseDrivers::SQLite3 => {
                    let pool = &self.sqlite.clone().unwrap().pool;

                    let mut blacklists_transaction = pool.begin().await?;
                    let mut blacklists_handled_entries = 0u64;
                    let mut blacklists_insert_entries = Vec::new();
                    let query = format!("DELETE FROM {}", self.config.db_structure.db_blacklist);
                    sqlx::query(&query).execute(&mut blacklists_transaction).await?;
                    for info_hash in blacklists.iter() {
                        blacklists_handled_entries += 1;
                        blacklists_insert_entries.push(format!("('{}')", info_hash).to_string());
                        if blacklists_insert_entries.len() == 10000 {
                            let pre_query = format!(
                                "INSERT OR REPLACE INTO {} ({}) VALUES",
                                self.config.db_structure.db_blacklist,
                                self.config.db_structure.table_blacklist_info_hash
                            );
                            let query = format!(
                                "{} {}",
                                pre_query,
                                blacklists_insert_entries.join(",")
                            );
                            sqlx::query(&query).execute(&mut blacklists_transaction).await?;
                            info!("[SQLite3] Handled {} blacklists", blacklists_handled_entries);
                            blacklists_insert_entries = vec![];
                        }
                    }
                    if !blacklists_insert_entries.is_empty() {
                        let pre_query = format!(
                            "INSERT OR REPLACE INTO {} ({}) VALUES",
                            self.config.db_structure.db_blacklist,
                            self.config.db_structure.table_blacklist_info_hash
                        );
                        let query = format!(
                            "{} {}",
                            pre_query,
                            blacklists_insert_entries.join(",")
                        );
                        sqlx::query(&query).execute(&mut blacklists_transaction).await?;
                        info!("[SQLite3] Handled {} whitelists", blacklists_handled_entries);
                    }
                    match blacklists_transaction.commit().await {
                        Ok(_) => {}
                        Err(e) => {
                            error!("[SQLite3] Error: {}", e.to_string());
                            return Err(e);
                        }
                    };

                    Ok(())
                }
                DatabaseDrivers::MySQL => {
                    let pool = &self.mysql.clone().unwrap().pool;

                    let mut blacklists_transaction = pool.begin().await?;
                    let mut blacklists_handled_entries = 0u64;
                    let mut blacklists_insert_entries = Vec::new();
                    let query = format!("TRUNCATE TABLE {}", self.config.db_structure.db_blacklist);
                    sqlx::query(&query).execute(&mut blacklists_transaction).await?;
                    for info_hash in blacklists.iter() {
                        blacklists_handled_entries += 1;
                        blacklists_insert_entries.push(format!("(UNHEX(\"{}\"))", info_hash).to_string());
                        if blacklists_insert_entries.len() == 10000 {
                            let query = format!(
                                "INSERT INTO {} (`{}`) VALUES {}",
                                self.config.db_structure.db_blacklist,
                                self.config.db_structure.table_blacklist_info_hash,
                                blacklists_insert_entries.join(",")
                            );
                            sqlx::query(&query).execute(&mut blacklists_transaction).await?;
                            info!("[MySQL] Handled {} blacklists", blacklists_handled_entries);
                            blacklists_insert_entries = vec![];
                        }
                    }
                    if !blacklists_insert_entries.is_empty() {
                        let query = format!(
                            "INSERT INTO {} (`{}`) VALUES {}",
                            self.config.db_structure.db_blacklist,
                            self.config.db_structure.table_blacklist_info_hash,
                            blacklists_insert_entries.join(",")
                        );
                        sqlx::query(&query).execute(&mut blacklists_transaction).await?;
                        info!("[MySQL] Handled {} blacklists", blacklists_handled_entries);
                    }
                    match blacklists_transaction.commit().await {
                        Ok(_) => {}
                        Err(e) => {
                            error!("[MySQL] Error: {}", e.to_string());
                            return Err(e);
                        }
                    };

                    Ok(())
                }
                DatabaseDrivers::PgSQL => {
                    let pool = &self.pgsql.clone().unwrap().pool;

                    let mut blacklists_transaction = pool.begin().await?;
                    let mut blacklists_handled_entries = 0u64;
                    let mut blacklists_insert_entries = Vec::new();
                    let query = format!("TRUNCATE TABLE {} RESTART IDENTITY", self.config.db_structure.db_blacklist);
                    sqlx::query(&query).execute(&mut blacklists_transaction).await?;
                    for info_hash in blacklists.iter() {
                        blacklists_handled_entries += 1;
                        blacklists_insert_entries.push(format!("(decode('{}', 'hex'))", info_hash).to_string());
                        if blacklists_insert_entries.len() == 10000 {
                            let query = format!(
                                "INSERT INTO {} ({}) VALUES {}",
                                self.config.db_structure.db_blacklist,
                                self.config.db_structure.table_blacklist_info_hash,
                                blacklists_insert_entries.join(",")
                            );
                            sqlx::query(&query).execute(&mut blacklists_transaction).await?;
                            info!("[PgSQL] Handled {} blacklists", blacklists_handled_entries);
                            blacklists_insert_entries = vec![];
                        }
                    }
                    if !blacklists_insert_entries.is_empty() {
                        let query = format!(
                            "INSERT INTO {} ({}) VALUES {}",
                            self.config.db_structure.db_blacklist,
                            self.config.db_structure.table_blacklist_info_hash,
                            blacklists_insert_entries.join(",")
                        );
                        sqlx::query(&query).execute(&mut blacklists_transaction).await?;
                        info!("[PgSQL] Handled {} blacklists", blacklists_handled_entries);
                    }
                    match blacklists_transaction.commit().await {
                        Ok(_) => {}
                        Err(e) => {
                            error!("[PgSQL] Error: {}", e.to_string());
                            return Err(e);
                        }
                    };

                    Ok(())
                }
            }
        }

        Err(Error::RowNotFound)
    }

    pub async fn save_torrents(&self, torrents: HashMap<InfoHash, i64>) -> Result<(), Error>
    {
        if self.engine.is_some() {
            return match self.engine.clone().unwrap() {
                DatabaseDrivers::SQLite3 => {
                    let pool = &self.sqlite.clone().unwrap().pool;

                    let mut torrents_transaction = pool.begin().await?;
                    let mut torrents_handled_entries = 0u64;
                    let mut torrents_insert_entries = Vec::new();
                    for (info_hash, completed) in torrents.iter() {
                        torrents_handled_entries += 1;
                        torrents_insert_entries.push(format!("('{}',{})", info_hash, completed.clone()).to_string());
                        if torrents_insert_entries.len() == 10000 {
                            let pre_query = format!(
                                "INSERT OR REPLACE INTO {} ({},{}) VALUES",
                                self.config.db_structure.db_torrents,
                                self.config.db_structure.table_torrents_info_hash,
                                self.config.db_structure.table_torrents_completed
                            );
                            let query = format!(
                                "{} {}",
                                pre_query,
                                torrents_insert_entries.join(",")
                            );
                            sqlx::query(&query).execute(&mut torrents_transaction).await?;
                            info!("[SQLite3] Handled {} torrents", torrents_handled_entries);
                            torrents_insert_entries = vec![];
                        }
                    }
                    if !torrents_insert_entries.is_empty() {
                        let pre_query = format!(
                            "INSERT OR REPLACE INTO {} ({},{}) VALUES",
                            self.config.db_structure.db_torrents,
                            self.config.db_structure.table_torrents_info_hash,
                            self.config.db_structure.table_torrents_completed
                        );
                        let query = format!(
                            "{} {}",
                            pre_query,
                            torrents_insert_entries.join(",")
                        );
                        sqlx::query(&query).execute(&mut torrents_transaction).await?;
                        info!("[SQLite3] Handled {} torrents", torrents_handled_entries);
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
                DatabaseDrivers::MySQL => {
                    let pool = &self.mysql.clone().unwrap().pool;

                    let mut torrents_transaction = pool.begin().await?;
                    let mut torrents_handled_entries = 0u64;
                    let mut torrents_insert_entries = Vec::new();
                    for (info_hash, completed) in torrents.iter() {
                        torrents_handled_entries += 1;
                        torrents_insert_entries.push(format!("(UNHEX(\"{}\"),{})", info_hash, completed.clone()).to_string());
                        if torrents_insert_entries.len() == 10000 {
                            let query = format!(
                                "INSERT INTO {} (`{}`,`{}`) VALUES {} ON DUPLICATE KEY UPDATE `{}`=VALUES(`{}`)",
                                self.config.db_structure.db_torrents,
                                self.config.db_structure.table_torrents_info_hash,
                                self.config.db_structure.table_torrents_completed,
                                torrents_insert_entries.join(","),
                                self.config.db_structure.table_torrents_completed,
                                self.config.db_structure.table_torrents_completed
                            );
                            sqlx::query(&query).execute(&mut torrents_transaction).await?;
                            info!("[MySQL] Handled {} torrents", torrents_handled_entries);
                            torrents_insert_entries = vec![];
                        }
                    }
                    if !torrents_insert_entries.is_empty() {
                        let query = format!(
                            "INSERT INTO {} (`{}`,`{}`) VALUES {} ON DUPLICATE KEY UPDATE `{}`=VALUES(`{}`)",
                            self.config.db_structure.db_torrents,
                            self.config.db_structure.table_torrents_info_hash,
                            self.config.db_structure.table_torrents_completed,
                            torrents_insert_entries.join(","),
                            self.config.db_structure.table_torrents_completed,
                            self.config.db_structure.table_torrents_completed
                        );
                        sqlx::query(&query).execute(&mut torrents_transaction).await?;
                        info!("[MySQL] Handled {} torrents", torrents_handled_entries);
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
                DatabaseDrivers::PgSQL => {
                    let pool = &self.pgsql.clone().unwrap().pool;

                    let mut torrents_transaction = pool.begin().await?;
                    let mut torrents_handled_entries = 0u64;
                    let mut torrents_insert_entries = Vec::new();
                    for (info_hash, completed) in torrents.iter() {
                        torrents_handled_entries += 1;
                        torrents_insert_entries.push(format!("(decode('{}', 'hex'),{})", info_hash, completed.clone()).to_string());
                        if torrents_insert_entries.len() == 10000 {
                            let query = format!(
                                "INSERT INTO {} ({},{}) VALUES {} ON CONFLICT ({}) DO UPDATE SET {}=excluded.{}",
                                self.config.db_structure.db_torrents,
                                self.config.db_structure.table_torrents_info_hash,
                                self.config.db_structure.table_torrents_completed,
                                torrents_insert_entries.join(","),
                                self.config.db_structure.table_torrents_info_hash,
                                self.config.db_structure.table_torrents_completed,
                                self.config.db_structure.table_torrents_completed
                            );
                            sqlx::query(&query).execute(&mut torrents_transaction).await?;
                            info!("[PgSQL] Handled {} torrents", torrents_handled_entries);
                            torrents_insert_entries = vec![];
                        }
                    }
                    if !torrents_insert_entries.is_empty() {
                        let query = format!(
                            "INSERT INTO {} ({},{}) VALUES {} ON CONFLICT ({}) DO UPDATE SET {}=excluded.{}",
                            self.config.db_structure.db_torrents,
                            self.config.db_structure.table_torrents_info_hash,
                            self.config.db_structure.table_torrents_completed,
                            torrents_insert_entries.join(","),
                            self.config.db_structure.table_torrents_info_hash,
                            self.config.db_structure.table_torrents_completed,
                            self.config.db_structure.table_torrents_completed
                        );
                        sqlx::query(&query).execute(&mut torrents_transaction).await?;
                        info!("[PgSQL] Handled {} torrents", torrents_handled_entries);
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
            }
        }

        Err(Error::RowNotFound)
    }
}