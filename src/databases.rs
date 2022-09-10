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
                        config.db_structure.table_info_hash,
                        config.db_structure.table_completed
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

    pub async fn load_torrents(&self) -> Result<Vec<(InfoHash, i64)>, sqlx::Error>
    {
        let mut return_data = vec![];
        let mut counter = 0u64;
        let mut total = 0u64;
        if self.engine.is_some() {
            match self.engine.clone().unwrap() {
                DatabaseDrivers::SQLite3 => {
                    let pool = &self.sqlite.clone().unwrap().pool;
                    // let mut rows = sqlx::query("SELECT info_hash,completed FROM torrents").fetch(pool);
                    let query = format!(
                        "SELECT {},{} FROM {}",
                        self.config.db_structure.table_info_hash,
                        self.config.db_structure.table_completed,
                        self.config.db_structure.db_torrents
                    );
                    let mut rows = sqlx::query(
                        query.as_str()
                    ).fetch(pool);
                    while let Some(result) = rows.try_next().await? {
                        if counter == 10000 {
                            info!("[SQLite3] Loaded {} torrents...", total);
                            counter = 0;
                        }
                        let infohash_data: &str = result.get(self.config.db_structure.table_info_hash.clone().as_str());
                        let infohash_decoded = hex::decode(infohash_data).unwrap();
                        let completed_data: i64 = result.get(self.config.db_structure.table_completed.clone().as_str());
                        let infohash = <[u8; 20]>::try_from(infohash_decoded[0 .. 20].as_ref()).unwrap();
                        return_data.push((InfoHash(infohash), completed_data));
                        counter += 1;
                        total += 1;
                    }
                    info!("[SQLite3] Loaded {} torrents...", total);
                    info!("[SQLite3] Loading completed !");
                    return Ok(return_data);
                }
                DatabaseDrivers::MySQL => {
                    let pool = &self.mysql.clone().unwrap().pool;
                    // let mut rows = sqlx::query("SELECT `info_hash`,`completed` FROM `torrents`").fetch(pool);
                    let query = format!(
                        "SELECT `{}`,`{}` FROM `{}`",
                        self.config.db_structure.table_info_hash,
                        self.config.db_structure.table_completed,
                        self.config.db_structure.db_torrents
                    );
                    let mut rows = sqlx::query(
                        query.as_str()
                    ).fetch(pool);
                    while let Some(result) = rows.try_next().await? {
                        if counter == 10000 {
                            info!("[MySQL] Loaded {} torrents...", total);
                            counter = 0;
                        }
                        let infohash_data: &[u8] = result.get(self.config.db_structure.table_info_hash.clone().as_str());
                        let completed_data: i64 = result.get(self.config.db_structure.table_completed.clone().as_str());
                        let infohash = <[u8; 20]>::try_from(infohash_data[0 .. 20].as_ref()).unwrap();
                        return_data.push((InfoHash(infohash), completed_data));
                        counter += 1;
                        total += 1;
                    }
                    info!("[MySQL] Loaded {} torrents...", total);
                    info!("[MySQL] Loading completed !");
                    return Ok(return_data);
                }
                DatabaseDrivers::PgSQL => {
                    let pool = &self.pgsql.clone().unwrap().pool;
                    // let mut rows = sqlx::query("SELECT info_hash, completed FROM torrents").fetch(pool);
                    let query = format!(
                        "SELECT {},{} FROM {}",
                        self.config.db_structure.table_info_hash,
                        self.config.db_structure.table_completed,
                        self.config.db_structure.db_torrents
                    );
                    let mut rows = sqlx::query(
                        query.as_str()
                    ).fetch(pool);
                    while let Some(result) = rows.try_next().await? {
                        if counter == 10000 {
                            info!("[PgSQL] Loaded {} torrents...", total);
                            counter = 0;
                        }
                        let infohash_data: &[u8] = result.get(self.config.db_structure.table_info_hash.clone().as_str());
                        let completed_data: i64 = result.get(self.config.db_structure.table_completed.clone().as_str());
                        let infohash = <[u8; 20]>::try_from(infohash_data[0 .. 20].as_ref()).unwrap();
                        return_data.push((InfoHash(infohash), completed_data));
                        counter += 1;
                        total += 1;
                    }
                    info!("[PgSQL] Loaded {} torrents...", total);
                    info!("[PgSQL] Loading completed !");
                    return Ok(return_data);
                }
            }
        }

        Err(sqlx::Error::RowNotFound)
    }

    pub async fn save_torrents(&self, torrents: HashMap<InfoHash, i64>) -> Result<(), sqlx::Error>
    {
        if self.engine.is_some() {
            match self.engine.clone().unwrap() {
                DatabaseDrivers::SQLite3 => {
                    let pool = &self.sqlite.clone().unwrap().pool;
                    let mut transaction = pool.begin().await?;
                    let mut handled_entries = 0u64;
                    let mut insert_entries = Vec::new();
                    for (info_hash, completed) in torrents.iter() {
                        handled_entries += 1;
                        insert_entries.push(format!("('{}',{})", info_hash, completed.clone()).to_string());
                        if insert_entries.len() == 10000 {
                            let pre_query = format!(
                                "INSERT OR REPLACE INTO {} ({},{}) VALUES",
                                self.config.db_structure.db_torrents,
                                self.config.db_structure.table_info_hash,
                                self.config.db_structure.table_completed
                            );
                            let query = format!(
                                "{} {}",
                                pre_query,
                                insert_entries.join(",")
                            );
                            sqlx::query(&query).execute(&mut transaction).await?;
                            info!("[SQLite3] Handled {} torrents", handled_entries);
                            insert_entries = vec![];
                        }
                    }
                    if !insert_entries.is_empty() {
                        let pre_query = format!(
                            "INSERT OR REPLACE INTO {} ({},{}) VALUES",
                            self.config.db_structure.db_torrents,
                            self.config.db_structure.table_info_hash,
                            self.config.db_structure.table_completed
                        );
                        let query = format!(
                            "{} {}",
                            pre_query,
                            insert_entries.join(",")
                        );
                        sqlx::query(&query).execute(&mut transaction).await?;
                        info!("[SQLite3] Handled {} torrents", handled_entries);
                    }
                    match transaction.commit().await {
                        Ok(_) => {}
                        Err(e) => {
                            error!("[SQLite3] Error: {}", e.to_string());
                            return Err(e);
                        }
                    };
                    return Ok(());
                }
                DatabaseDrivers::MySQL => {
                    let pool = &self.mysql.clone().unwrap().pool;
                    let mut transaction = pool.begin().await?;
                    let mut handled_entries = 0u64;
                    let mut insert_entries = Vec::new();
                    for (info_hash, completed) in torrents.iter() {
                        handled_entries += 1;
                        insert_entries.push(format!("(UNHEX(\"{}\"),{})", info_hash, completed.clone()).to_string());
                        if insert_entries.len() == 10000 {
                            let query = format!(
                                "INSERT INTO {} (`{}`,`{}`) VALUES {} ON DUPLICATE KEY UPDATE `{}`=VALUES(`{}`)",
                                self.config.db_structure.db_torrents,
                                self.config.db_structure.table_info_hash,
                                self.config.db_structure.table_completed,
                                insert_entries.join(","),
                                self.config.db_structure.table_completed,
                                self.config.db_structure.table_completed
                            );
                            sqlx::query(&query).execute(&mut transaction).await?;
                            info!("[MySQL] Handled {} torrents", handled_entries);
                            insert_entries = vec![];
                        }
                    }
                    if !insert_entries.is_empty() {
                        let query = format!(
                            "INSERT INTO {} (`{}`,`{}`) VALUES {} ON DUPLICATE KEY UPDATE `{}`=VALUES(`{}`)",
                            self.config.db_structure.db_torrents,
                            self.config.db_structure.table_info_hash,
                            self.config.db_structure.table_completed,
                            insert_entries.join(","),
                            self.config.db_structure.table_completed,
                            self.config.db_structure.table_completed
                        );
                        sqlx::query(&query).execute(&mut transaction).await?;
                        info!("[MySQL] Handled {} torrents", handled_entries);
                    }
                    match transaction.commit().await {
                        Ok(_) => {}
                        Err(e) => {
                            error!("[MySQL] Error: {}", e.to_string());
                            return Err(e);
                        }
                    };
                    return Ok(());
                }
                DatabaseDrivers::PgSQL => {
                    let pool = &self.pgsql.clone().unwrap().pool;
                    let mut transaction = pool.begin().await?;
                    let mut handled_entries = 0u64;
                    let mut insert_entries = Vec::new();
                    for (info_hash, completed) in torrents.iter() {
                        handled_entries += 1;
                        insert_entries.push(format!("(decode('{}', 'hex'),{})", info_hash, completed.clone()).to_string());
                        if insert_entries.len() == 10000 {
                            let query = format!(
                                "INSERT INTO {} ({},{}) VALUES {} ON CONFLICT ({}) DO UPDATE SET {}=excluded.{}",
                                self.config.db_structure.db_torrents,
                                self.config.db_structure.table_info_hash,
                                self.config.db_structure.table_completed,
                                insert_entries.join(","),
                                self.config.db_structure.table_info_hash,
                                self.config.db_structure.table_completed,
                                self.config.db_structure.table_completed
                            );
                            sqlx::query(&query).execute(&mut transaction).await?;
                            info!("[PgSQL] Handled {} torrents", handled_entries);
                            insert_entries = vec![];
                        }
                    }
                    if !insert_entries.is_empty() {
                        let query = format!(
                            "INSERT INTO {} ({},{}) VALUES {} ON CONFLICT ({}) DO UPDATE SET {}=excluded.{}",
                            self.config.db_structure.db_torrents,
                            self.config.db_structure.table_info_hash,
                            self.config.db_structure.table_completed,
                            insert_entries.join(","),
                            self.config.db_structure.table_info_hash,
                            self.config.db_structure.table_completed,
                            self.config.db_structure.table_completed
                        );
                        sqlx::query(&query).execute(&mut transaction).await?;
                        info!("[PgSQL] Handled {} torrents", handled_entries);
                    }
                    match transaction.commit().await {
                        Ok(_) => {}
                        Err(e) => {
                            error!("[PgSQL] Error: {}", e.to_string());
                            return Err(e);
                        }
                    };
                    return Ok(());
                }
            }
        }

        Err(sqlx::Error::RowNotFound)
    }
}