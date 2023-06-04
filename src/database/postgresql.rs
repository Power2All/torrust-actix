use futures_util::TryStreamExt;
use log::{error, info};
use scc::ebr::Arc;
use sqlx::{ConnectOptions, Error, Pool, Postgres, Row};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use crate::common::InfoHash;
use crate::config::Configuration;
use crate::databases::{DatabaseConnector, DatabaseDrivers};
use crate::tracker::TorrentTracker;
use crate::tracker_objects::torrents::TorrentEntryItem;

#[derive(Clone)]
pub struct DatabaseConnectorPgSQL {
    pool: Pool<Postgres>,
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

    pub async fn database_connector(config: Arc<Configuration>) -> DatabaseConnector
    {
        let pgsql_connect = DatabaseConnectorPgSQL::create(&config.db_path).await;
        if pgsql_connect.is_err() {
            error!("[PgSQL] Unable to connect to PostgresSQL on DSL {}", &config.db_path)
        }

        let mut structure = DatabaseConnector { mysql: None, sqlite: None, pgsql: None, engine: None };
        structure.pgsql = Some(DatabaseConnectorPgSQL { pool: pgsql_connect.unwrap() });
        structure.engine = Some(DatabaseDrivers::mysql);

        structure
    }

    pub async fn load_torrents(&self, tracker: Arc<TorrentTracker>) -> Result<(u64, u64), Error>
    {
        let mut counter = 0u64;
        let mut total_torrents = 0u64;
        let mut total_completes = 0u64;

        let query = format!(
            "SELECT {},{} FROM {}",
            tracker.config.db_structure.table_torrents_info_hash,
            tracker.config.db_structure.table_torrents_completed,
            tracker.config.db_structure.db_torrents
        );
        let mut rows = sqlx::query(query.as_str()).fetch(&self.pool);
        let mut torrents_parsing = HashMap::new();
        while let Some(result) = rows.try_next().await? {
            if counter == 100000 {
                tracker.add_torrents(torrents_parsing.clone(), false).await;
                torrents_parsing.clear();
                info!("[PgSQL] Loaded {} torrents...", total_torrents);
                counter = 0;
            }
            let info_hash_data: &[u8] = result.get(tracker.config.db_structure.table_torrents_info_hash.clone().as_str());
            let info_hash_decoded = hex::decode(info_hash_data).unwrap();
            let completed_data: i64 = result.get(tracker.config.db_structure.table_torrents_completed.clone().as_str());
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

    pub async fn save_torrents(&self, tracker: Arc<TorrentTracker>, torrents: HashMap<InfoHash, i64>) -> Result<(), Error>
    {
        let mut torrents_transaction = self.pool.begin().await?;
        let mut torrents_handled_entries = 0u64;
        for (info_hash, completed) in torrents.iter() {
            torrents_handled_entries += 1;
            match sqlx::query(&format!(
                "INSERT INTO {} ({},{}) VALUES ('{}',{}) ON CONFLICT ({}) DO UPDATE SET {}=excluded.{}",
                tracker.config.db_structure.db_torrents,
                tracker.config.db_structure.table_torrents_info_hash,
                tracker.config.db_structure.table_torrents_completed,
                info_hash,
                completed.clone(),
                tracker.config.db_structure.table_torrents_info_hash,
                tracker.config.db_structure.table_torrents_completed,
                tracker.config.db_structure.table_torrents_completed
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
                info!("[PgSQL] Loaded {} whitelists...", total_whitelist);
                counter = 0;
            }
            let info_hash_data: &[u8] = result.get(tracker.config.db_structure.table_whitelist_info_hash.clone().as_str());
            let info_hash_decoded = hex::decode(info_hash_data).unwrap();
            let info_hash = <[u8; 20]>::try_from(info_hash_decoded[0..20].as_ref()).unwrap();
            return_data_whitelist.push(InfoHash(info_hash));
            counter += 1;
            total_whitelist += 1;
        }

        info!("[PgSQL] Loaded {} whitelists...", total_whitelist);
        Ok(return_data_whitelist)
    }

    pub async fn save_whitelist(&self, tracker: Arc<TorrentTracker>, whitelists: Vec<(InfoHash, i64)>) -> Result<(), Error>
    {
        let mut whitelist_transaction = self.pool.begin().await?;
        let mut whitelist_handled_entries = 0u64;
        match sqlx::query(&format!("TRUNCATE TABLE {} RESTART IDENTITY", tracker.config.db_structure.db_whitelist)).execute(&mut whitelist_transaction).await {
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
                    tracker.config.db_structure.db_whitelist,
                    tracker.config.db_structure.table_whitelist_info_hash,
                    info_hash,
                    tracker.config.db_structure.table_whitelist_info_hash
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
                    tracker.config.db_structure.db_whitelist,
                    tracker.config.db_structure.table_whitelist_info_hash,
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
                info!("[PgSQL] Loaded {} blacklists...", total_blacklist);
                counter = 0;
            }
            let info_hash_data: &[u8] = result.get(tracker.config.db_structure.table_blacklist_info_hash.clone().as_str());
            let info_hash_decoded = hex::decode(info_hash_data).unwrap();
            let info_hash = <[u8; 20]>::try_from(info_hash_decoded[0..20].as_ref()).unwrap();
            return_data_blacklist.push(InfoHash(info_hash));
            counter += 1;
            total_blacklist += 1;
        }

        info!("[PgSQL] Loaded {} blacklists...", total_blacklist);
        Ok(return_data_blacklist)
    }

    pub async fn save_blacklist(&self, tracker: Arc<TorrentTracker>, blacklists: Vec<InfoHash>) -> Result<(), Error>
    {
        let mut blacklist_transaction = self.pool.begin().await?;
        let mut blacklist_handled_entries = 0u64;
        let _ = sqlx::query(&format!("TRUNCATE TABLE {} RESTART IDENTITY", tracker.config.db_structure.db_blacklist)).execute(&mut blacklist_transaction).await?;
        for info_hash in blacklists.iter() {
            blacklist_handled_entries += 1;
            match sqlx::query(&format!(
                "INSERT INTO {} ({}) VALUES ('{}')",
                tracker.config.db_structure.db_blacklist,
                tracker.config.db_structure.table_blacklist_info_hash,
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
                info!("[PgSQL] Loaded {} keys...", total_keys);
                counter = 0;
            }
            let hash_data: &[u8] = result.get(tracker.config.db_structure.table_keys_hash.clone().as_str());
            let hash_decoded = hex::decode(hash_data).unwrap();
            let timeout_data: i64 = result.get(tracker.config.db_structure.table_keys_timeout.clone().as_str());
            let hash = <[u8; 20]>::try_from(hash_decoded[0..20].as_ref()).unwrap();
            return_data_keys.push((InfoHash(hash), timeout_data));
            counter += 1;
            total_keys += 1;
        }

        info!("[PgSQL] Loaded {} keys...", total_keys);
        Ok(return_data_keys)
    }

    pub async fn save_keys(&self, tracker: Arc<TorrentTracker>, keys: Vec<(InfoHash, i64)>) -> Result<(), Error>
    {
        let mut keys_transaction = self.pool.begin().await?;
        let mut keys_handled_entries = 0u64;
        for (hash, timeout) in keys.iter() {
            keys_handled_entries += 1;
            match sqlx::query(&format!(
                "INSERT INTO {} ({},{}) VALUES ('{}',{}) ON CONFLICT ({}) DO UPDATE SET {}=excluded.{}",
                tracker.config.db_structure.db_keys,
                tracker.config.db_structure.table_keys_hash,
                tracker.config.db_structure.table_keys_timeout,
                hash,
                timeout.clone(),
                tracker.config.db_structure.table_keys_hash,
                tracker.config.db_structure.table_keys_timeout,
                tracker.config.db_structure.table_keys_timeout
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
}
