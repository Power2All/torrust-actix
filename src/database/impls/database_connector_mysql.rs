use crate::config::structs::configuration::Configuration;
use crate::database::enums::database_drivers::DatabaseDrivers;
use crate::database::structs::database_connector::DatabaseConnector;
use crate::database::structs::database_connector_mysql::DatabaseConnectorMySQL;
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_entry::AHashMap;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::tracker::structs::user_entry_item::UserEntryItem;
use crate::tracker::structs::user_id::UserId;
use async_std::task;
use futures_util::TryStreamExt;
use log::{error, info};
use sha1::{Digest, Sha1};
use sqlx::mysql::{MySqlConnectOptions, MySqlPoolOptions};
use sqlx::{ConnectOptions, Error, MySql, Pool, Row, Transaction};
use std::collections::BTreeMap;
use std::ops::Deref;
use std::process::exit;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

impl DatabaseConnectorMySQL {
    #[tracing::instrument(level = "debug")]
    pub async fn create(dsl: &str) -> Result<Pool<MySql>, Error>
    {
        MySqlPoolOptions::new().connect_with(
            MySqlConnectOptions::from_str(dsl)?
                .log_statements(log::LevelFilter::Debug)
                .log_slow_statements(log::LevelFilter::Debug, Duration::from_secs(1))
        ).await
    }

    #[tracing::instrument(level = "debug")]
    pub async fn database_connector(config: Arc<Configuration>, create_database: bool) -> DatabaseConnector
    {
        let mysql_connect = DatabaseConnectorMySQL::create(config.database.clone().path.as_str()).await;
        if let Err(mysql_connect) = mysql_connect {
            error!("[MySQL] Unable to connect to MySQL on DSL {}", config.database.clone().path);
            error!("[MySQL] Message: {:#?}", mysql_connect.into_database_error().unwrap().message());
            exit(1);
        }
        let mut structure = DatabaseConnector { mysql: None, sqlite: None, pgsql: None, engine: None };
        structure.mysql = Some(DatabaseConnectorMySQL { pool: mysql_connect.unwrap() });
        structure.engine = Some(DatabaseDrivers::mysql);
        if create_database {
            let pool = &structure.mysql.clone().unwrap().pool;
            info!("[BOOT] Database creation triggered for MySQL.");
            info!("[BOOT MySQL] Creating table {}", config.database_structure.clone().torrents.table_name);
            match config.database_structure.clone().torrents.bin_type_infohash {
                true => {
                    match sqlx::query(
                        format!(
                            "CREATE TABLE `{}` (`{}` BINARY(20) NOT NULL, `{}` INT NOT NULL DEFAULT 0, `{}` INT NOT NULL DEFAULT 0, `{}` BIGINT UNSIGNED NOT NULL DEFAULT 0, PRIMARY KEY (`{}`)) COLLATE='utf8mb4_general_ci'",
                            config.database_structure.clone().torrents.table_name,
                            config.database_structure.clone().torrents.column_infohash,
                            config.database_structure.clone().torrents.column_seeds,
                            config.database_structure.clone().torrents.column_peers,
                            config.database_structure.clone().torrents.column_completed,
                            config.database_structure.clone().torrents.column_infohash
                        ).as_str()
                    ).execute(pool).await {
                        Ok(_) => {}
                        Err(error) => { panic!("[MySQL] Error: {error}"); }
                    }
                }
                false => {
                    match sqlx::query(
                        format!(
                            "CREATE TABLE `{}` (`{}` VARCHAR(40) NOT NULL, `{}` INT NOT NULL DEFAULT 0, `{}` INT NOT NULL DEFAULT 0, `{}` BIGINT UNSIGNED NOT NULL DEFAULT 0, PRIMARY KEY (`{}`)) COLLATE='utf8mb4_general_ci'",
                            config.database_structure.clone().torrents.table_name,
                            config.database_structure.clone().torrents.column_infohash,
                            config.database_structure.clone().torrents.column_seeds,
                            config.database_structure.clone().torrents.column_peers,
                            config.database_structure.clone().torrents.column_completed,
                            config.database_structure.clone().torrents.column_infohash
                        ).as_str()
                    ).execute(pool).await {
                        Ok(_) => {}
                        Err(error) => { panic!("[MySQL] Error: {error}"); }
                    }
                }
            }
            info!("[BOOT MySQL] Creating table {}", config.database_structure.clone().whitelist.table_name);
            match config.database_structure.clone().whitelist.bin_type_infohash {
                true => {
                    match sqlx::query(
                        format!(
                            "CREATE TABLE `{}` (`{}` BINARY(20) NOT NULL, PRIMARY KEY (`{}`)) COLLATE='utf8mb4_general_ci'",
                            config.database_structure.clone().whitelist.table_name,
                            config.database_structure.clone().whitelist.column_infohash,
                            config.database_structure.clone().whitelist.column_infohash
                        ).as_str()
                    ).execute(pool).await {
                        Ok(_) => {}
                        Err(error) => { panic!("[MySQL] Error: {error}"); }
                    }
                }
                false => {
                    match sqlx::query(
                        format!(
                            "CREATE TABLE `{}` (`{}` VARCHAR(40) NOT NULL, PRIMARY KEY (`{}`)) COLLATE='utf8mb4_general_ci'",
                            config.database_structure.clone().whitelist.table_name,
                            config.database_structure.clone().whitelist.column_infohash,
                            config.database_structure.clone().whitelist.column_infohash
                        ).as_str()
                    ).execute(pool).await {
                        Ok(_) => {}
                        Err(error) => { panic!("[MySQL] Error: {error}"); }
                    }
                }
            }
            info!("[BOOT MySQL] Creating table {}", config.database_structure.clone().blacklist.table_name);
            match config.database_structure.clone().blacklist.bin_type_infohash {
                true => {
                    match sqlx::query(
                        format!(
                            "CREATE TABLE `{}` (`{}` BINARY(20) NOT NULL, PRIMARY KEY (`{}`)) COLLATE='utf8mb4_general_ci'",
                            config.database_structure.clone().blacklist.table_name,
                            config.database_structure.clone().blacklist.column_infohash,
                            config.database_structure.clone().blacklist.column_infohash
                        ).as_str()
                    ).execute(pool).await {
                        Ok(_) => {}
                        Err(error) => { panic!("[MySQL] Error: {error}"); }
                    }
                }
                false => {
                    match sqlx::query(
                        format!(
                            "CREATE TABLE `{}` (`{}` VARCHAR(40) NOT NULL, PRIMARY KEY (`{}`)) COLLATE='utf8mb4_general_ci'",
                            config.database_structure.clone().blacklist.table_name,
                            config.database_structure.clone().blacklist.column_infohash,
                            config.database_structure.clone().blacklist.column_infohash
                        ).as_str()
                    ).execute(pool).await {
                        Ok(_) => {}
                        Err(error) => { panic!("[MySQL] Error: {error}"); }
                    }
                }
            }
            info!("[BOOT MySQL] Creating table {}", config.database_structure.clone().keys.table_name);
            match config.database_structure.clone().keys.bin_type_hash {
                true => {
                    match sqlx::query(
                        format!(
                            "CREATE TABLE `{}` (`{}` BINARY(20) NOT NULL, `{}` INT NOT NULL DEFAULT 0, PRIMARY KEY (`{}`)) COLLATE='utf8mb4_general_ci'",
                            config.database_structure.clone().keys.table_name,
                            config.database_structure.clone().keys.column_hash,
                            config.database_structure.clone().keys.column_timeout,
                            config.database_structure.clone().keys.column_hash
                        ).as_str()
                    ).execute(pool).await {
                        Ok(_) => {}
                        Err(error) => { panic!("[MySQL] Error: {error}"); }
                    }
                }
                false => {
                    match sqlx::query(
                        format!(
                            "CREATE TABLE `{}` (`{}` VARCHAR(40) NOT NULL, `{}` INT NOT NULL DEFAULT 0, PRIMARY KEY (`{}`)) COLLATE='utf8mb4_general_ci'",
                            config.database_structure.clone().keys.table_name,
                            config.database_structure.clone().keys.column_hash,
                            config.database_structure.clone().keys.column_timeout,
                            config.database_structure.clone().keys.column_hash
                        ).as_str()
                    ).execute(pool).await {
                        Ok(_) => {}
                        Err(error) => { panic!("[MySQL] Error: {error}"); }
                    }
                }
            }
            info!("[BOOT MySQL] Creating table {}", config.database_structure.clone().users.table_name);
            match config.database_structure.clone().users.id_uuid {
                true => {
                    match config.database_structure.clone().users.bin_type_key {
                        true => {
                            match sqlx::query(
                                format!(
                                    "CREATE TABLE `{}` (`{}` VARCHAR(36) NOT NULL, `{}` BINARY(20) NOT NULL, `{}` BIGINT UNSIGNED NOT NULL DEFAULT 0, `{}` BIGINT UNSIGNED NOT NULL DEFAULT 0, `{}` BIGINT UNSIGNED NOT NULL DEFAULT 0, `{}` TINYINT NOT NULL DEFAULT 0, `{}` INT NOT NULL DEFAULT 0, PRIMARY KEY (`{}`)) COLLATE='utf8mb4_general_ci'",
                                    config.database_structure.clone().users.table_name,
                                    config.database_structure.clone().users.column_uuid,
                                    config.database_structure.clone().users.column_key,
                                    config.database_structure.clone().users.column_uploaded,
                                    config.database_structure.clone().users.column_downloaded,
                                    config.database_structure.clone().users.column_completed,
                                    config.database_structure.clone().users.column_active,
                                    config.database_structure.clone().users.column_updated,
                                    config.database_structure.clone().users.column_uuid
                                ).as_str()
                            ).execute(pool).await {
                                Ok(_) => {}
                                Err(error) => { panic!("[MySQL] Error: {error}"); }
                            }
                        }
                        false => {
                            match sqlx::query(
                                format!(
                                    "CREATE TABLE `{}` (`{}` VARCHAR(36) NOT NULL, `{}` VARCHAR(40) NOT NULL, `{}` BIGINT UNSIGNED NOT NULL DEFAULT 0, `{}` BIGINT UNSIGNED NOT NULL DEFAULT 0, `{}` BIGINT UNSIGNED NOT NULL DEFAULT 0, `{}` TINYINT NOT NULL DEFAULT 0, `{}` INT NOT NULL DEFAULT 0, PRIMARY KEY (`{}`)) COLLATE='utf8mb4_general_ci'",
                                    config.database_structure.clone().users.table_name,
                                    config.database_structure.clone().users.column_uuid,
                                    config.database_structure.clone().users.column_key,
                                    config.database_structure.clone().users.column_uploaded,
                                    config.database_structure.clone().users.column_downloaded,
                                    config.database_structure.clone().users.column_completed,
                                    config.database_structure.clone().users.column_active,
                                    config.database_structure.clone().users.column_updated,
                                    config.database_structure.clone().users.column_uuid
                                ).as_str()
                            ).execute(pool).await {
                                Ok(_) => {}
                                Err(error) => { panic!("[MySQL] Error: {error}"); }
                            }
                        }
                    }
                }
                false => {
                    match config.database_structure.clone().users.bin_type_key {
                        true => {
                            match sqlx::query(
                                format!(
                                    "CREATE TABLE `{}` (`{}` BIGINT UNSIGNED NOT NULL AUTO_INCREMENT, `{}` BINARY(20) NOT NULL, `{}` BIGINT UNSIGNED NOT NULL DEFAULT 0, `{}` BIGINT UNSIGNED NOT NULL DEFAULT 0, `{}` BIGINT UNSIGNED NOT NULL DEFAULT 0, `{}` TINYINT NOT NULL DEFAULT 0, `{}` INT NOT NULL DEFAULT 0, PRIMARY KEY (`{}`)) COLLATE='utf8mb4_general_ci'",
                                    config.database_structure.clone().users.table_name,
                                    config.database_structure.clone().users.column_id,
                                    config.database_structure.clone().users.column_key,
                                    config.database_structure.clone().users.column_uploaded,
                                    config.database_structure.clone().users.column_downloaded,
                                    config.database_structure.clone().users.column_completed,
                                    config.database_structure.clone().users.column_active,
                                    config.database_structure.clone().users.column_updated,
                                    config.database_structure.clone().users.column_id
                                ).as_str()
                            ).execute(pool).await {
                                Ok(_) => {}
                                Err(error) => { panic!("[MySQL] Error: {error}"); }
                            }
                        }
                        false => {
                            match sqlx::query(
                                format!(
                                    "CREATE TABLE `{}` (`{}` BIGINT UNSIGNED NOT NULL AUTO_INCREMENT, `{}` VARCHAR(40) NOT NULL, `{}` BIGINT UNSIGNED NOT NULL DEFAULT 0, `{}` BIGINT UNSIGNED NOT NULL DEFAULT 0, `{}` BIGINT UNSIGNED NOT NULL DEFAULT 0, `{}` TINYINT NOT NULL DEFAULT 0, `{}` INT NOT NULL DEFAULT 0, PRIMARY KEY (`{}`)) COLLATE='utf8mb4_general_ci'",
                                    config.database_structure.clone().users.table_name,
                                    config.database_structure.clone().users.column_id,
                                    config.database_structure.clone().users.column_key,
                                    config.database_structure.clone().users.column_uploaded,
                                    config.database_structure.clone().users.column_downloaded,
                                    config.database_structure.clone().users.column_completed,
                                    config.database_structure.clone().users.column_active,
                                    config.database_structure.clone().users.column_updated,
                                    config.database_structure.clone().users.column_id
                                ).as_str()
                            ).execute(pool).await {
                                Ok(_) => {}
                                Err(error) => { panic!("[MySQL] Error: {error}"); }
                            }
                        }
                    }
                }
            }
            info!("[BOOT] Created the database and tables, restart without the parameter to start the app.");
            task::sleep(Duration::from_secs(1)).await;
            exit(0);
        }
        structure
    }

    #[tracing::instrument(level = "debug")]
    pub async fn load_torrents(&self, tracker: Arc<TorrentTracker>) -> Result<(u64, u64), Error>
    {
        let mut start = 0u64;
        let length = 100000u64;
        let mut torrents = 0u64;
        let mut completed = 0u64;
        let structure = tracker.config.deref().clone().database_structure.clone().torrents;
        loop {
            let string_format = match tracker.config.deref().clone().database_structure.torrents.bin_type_infohash {
                true => {
                    format!(
                        "SELECT HEX(`{}`) AS `{}`, `{}` FROM `{}` LIMIT {}, {}",
                        structure.column_infohash,
                        structure.column_infohash,
                        structure.column_completed,
                        structure.table_name,
                        start,
                        length
                    )
                }
                false => {
                    format!(
                        "SELECT `{}`, `{}` FROM `{}` LIMIT {}, {}",
                        structure.column_infohash,
                        structure.column_completed,
                        structure.table_name,
                        start,
                        length
                    )
                }
            };
            let mut rows = sqlx::query(string_format.as_str()).fetch(&self.pool);
            while let Some(result) = rows.try_next().await? {
                let info_hash_data: &[u8] = result.get(structure.column_infohash.as_str());
                let info_hash: [u8; 20] = <[u8; 20]>::try_from(hex::decode(info_hash_data).unwrap()[0..20].as_ref()).unwrap();
                let completed_count: u64 = result.get(structure.column_completed.as_str());
                tracker.add_torrent(
                    InfoHash(info_hash),
                    TorrentEntry {
                        seeds: AHashMap::default(),
                        peers: AHashMap::default(),
                        completed: completed_count,
                        updated: std::time::Instant::now()
                    }
                );
                torrents += 1;
                completed += completed_count;
            }
            start += length;
            if torrents < start {
                break;
            }
            info!("[MySQL] Handled {torrents} torrents");
        }
        tracker.set_stats(StatsEvent::Completed, completed as i64);
        info!("[MySQL] Loaded {torrents} torrents with {completed} completed");
        Ok((torrents, completed))
    }

    #[tracing::instrument(level = "debug")]
    pub async fn save_torrents(&self, tracker: Arc<TorrentTracker>, torrents: BTreeMap<InfoHash, (TorrentEntry, UpdatesAction)>) -> Result<(), Error>
    {
        let mut torrents_transaction = self.pool.begin().await?;
        let mut torrents_handled_entries = 0u64;
        let structure = tracker.config.deref().clone().database_structure.clone().torrents;
        for (info_hash, (torrent_entry, updates_action)) in torrents.iter() {
            torrents_handled_entries += 1;
            match updates_action {
                UpdatesAction::Remove => {
                    if tracker.config.deref().clone().database.remove_action {
                        let string_format = match tracker.config.deref().clone().database_structure.torrents.bin_type_infohash {
                            true => {
                                format!(
                                    "DELETE FROM `{}` WHERE `{}`=UNHEX('{}')",
                                    structure.table_name,
                                    structure.column_infohash,
                                    info_hash
                                )
                            }
                            false => {
                                format!(
                                    "DELETE FROM `{}` WHERE `{}`='{}'",
                                    structure.table_name,
                                    structure.column_infohash,
                                    info_hash
                                )
                            }
                        };
                        match sqlx::query(string_format.as_str()).execute(&mut *torrents_transaction).await {
                            Ok(_) => {}
                            Err(e) => {
                                error!("[MySQL] Error: {e}");
                                return Err(e);
                            }
                        }
                    }
                }
                UpdatesAction::Add | UpdatesAction::Update => {
                    match tracker.config.deref().clone().database.insert_vacant {
                        true => {
                            if tracker.config.deref().clone().database.update_peers {
                                let string_format = match tracker.config.deref().clone().database_structure.torrents.bin_type_infohash {
                                    true => {
                                        format!(
                                            "INSERT INTO `{}` (`{}`, `{}`, `{}`) VALUES (UNHEX('{}'), {}, {}) ON DUPLICATE KEY UPDATE `{}` = VALUES(`{}`), `{}`=VALUES(`{}`)",
                                            structure.table_name,
                                            structure.column_infohash,
                                            structure.column_seeds,
                                            structure.column_peers,
                                            info_hash,
                                            torrent_entry.seeds.len(),
                                            torrent_entry.peers.len(),
                                            structure.column_seeds,
                                            structure.column_seeds,
                                            structure.column_peers,
                                            structure.column_peers
                                        )
                                    }
                                    false => {
                                        format!(
                                            "INSERT INTO `{}` (`{}`, `{}`, `{}`) VALUES ('{}', {}, {}) ON DUPLICATE KEY UPDATE `{}`=VALUES(`{}`), `{}`=VALUES(`{}`)",
                                            structure.table_name,
                                            structure.column_infohash,
                                            structure.column_seeds,
                                            structure.column_peers,
                                            info_hash,
                                            torrent_entry.seeds.len(),
                                            torrent_entry.peers.len(),
                                            structure.column_seeds,
                                            structure.column_seeds,
                                            structure.column_peers,
                                            structure.column_peers
                                        )
                                    }
                                };
                                match sqlx::query(string_format.as_str()).execute(&mut *torrents_transaction).await {
                                    Ok(_) => {}
                                    Err(e) => {
                                        error!("[MySQL] Error: {e}");
                                        return Err(e);
                                    }
                                }
                            }
                            if tracker.config.deref().clone().database.update_completed {
                                let string_format = match tracker.config.deref().clone().database_structure.torrents.bin_type_infohash {
                                    true => {
                                        format!(
                                            "INSERT INTO `{}` (`{}`, `{}`) VALUES (UNHEX('{}'), {}) ON DUPLICATE KEY UPDATE `{}`=VALUES(`{}`)",
                                            structure.table_name,
                                            structure.column_infohash,
                                            structure.column_completed,
                                            info_hash,
                                            torrent_entry.completed,
                                            structure.column_completed,
                                            structure.column_completed
                                        )
                                    }
                                    false => {
                                        format!(
                                            "INSERT INTO `{}` (`{}`, `{}`) VALUES ('{}', {}) ON DUPLICATE KEY UPDATE `{}`=VALUES(`{}`)",
                                            structure.table_name,
                                            structure.column_infohash,
                                            structure.column_completed,
                                            info_hash,
                                            torrent_entry.completed,
                                            structure.column_completed,
                                            structure.column_completed
                                        )
                                    }
                                };
                                match sqlx::query(string_format.as_str()).execute(&mut *torrents_transaction).await {
                                    Ok(_) => {}
                                    Err(e) => {
                                        error!("[MySQL] Error: {e}");
                                        return Err(e);
                                    }
                                }
                            }
                        }
                        false => {
                            if tracker.config.deref().clone().database.update_peers {
                                let string_format = match tracker.config.deref().clone().database_structure.torrents.bin_type_infohash {
                                    true => {
                                        format!(
                                            "UPDATE IGNORE `{}` SET `{}`={}, `{}`={} WHERE `{}`=UNHEX('{}')",
                                            structure.table_name,
                                            structure.column_seeds,
                                            torrent_entry.seeds.len(),
                                            structure.column_peers,
                                            torrent_entry.peers.len(),
                                            structure.column_infohash,
                                            info_hash
                                        )
                                    }
                                    false => {
                                        format!(
                                            "UPDATE IGNORE `{}` SET `{}`={}, `{}`={} WHERE `{}`='{}'",
                                            structure.table_name,
                                            structure.column_seeds,
                                            torrent_entry.seeds.len(),
                                            structure.column_peers,
                                            torrent_entry.peers.len(),
                                            structure.column_infohash,
                                            info_hash
                                        )
                                    }
                                };
                                match sqlx::query(string_format.as_str()).execute(&mut *torrents_transaction).await {
                                    Ok(_) => {}
                                    Err(e) => {
                                        error!("[MySQL] Error: {e}");
                                        return Err(e);
                                    }
                                }
                            }
                            if tracker.config.deref().clone().database.update_completed {
                                let string_format = match tracker.config.deref().clone().database_structure.torrents.bin_type_infohash {
                                    true => {
                                        format!(
                                            "UPDATE IGNORE `{}` SET `{}`={} WHERE `{}`=UNHEX('{}')",
                                            structure.table_name,
                                            structure.column_completed,
                                            torrent_entry.completed,
                                            structure.column_infohash,
                                            info_hash
                                        )
                                    }
                                    false => {
                                        format!(
                                            "UPDATE IGNORE `{}` SET `{}`={} WHERE `{}`='{}'",
                                            structure.table_name,
                                            structure.column_completed,
                                            torrent_entry.completed,
                                            structure.column_infohash,
                                            info_hash
                                        )
                                    }
                                };
                                match sqlx::query(string_format.as_str()).execute(&mut *torrents_transaction).await {
                                    Ok(_) => {}
                                    Err(e) => {
                                        error!("[MySQL] Error: {e}");
                                        return Err(e);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if (torrents_handled_entries as f64 / 1000f64).fract() == 0.0 || torrents.len() as u64 == torrents_handled_entries {
                info!("[MySQL] Handled {torrents_handled_entries} torrents");
            }
        }
        info!("[MySQL] Handled {torrents_handled_entries} torrents");
        self.commit(torrents_transaction).await
    }

    #[tracing::instrument(level = "debug")]
    pub async fn load_whitelist(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error>
    {
        let mut start = 0u64;
        let length = 100000u64;
        let mut hashes = 0u64;
        let structure = tracker.config.deref().clone().database_structure.clone().whitelist;
        loop {
            let string_format = match tracker.config.deref().clone().database_structure.whitelist.bin_type_infohash {
                true => {
                    format!(
                        "SELECT HEX(`{}`) AS `{}` FROM `{}` LIMIT {}, {}",
                        structure.column_infohash,
                        structure.column_infohash,
                        structure.table_name,
                        start,
                        length
                    )
                }
                false => {
                    format!(
                        "SELECT `{}` FROM `{}` LIMIT {}, {}",
                        structure.column_infohash,
                        structure.table_name,
                        start,
                        length
                    )
                }
            };
            let mut rows = sqlx::query(string_format.as_str()).fetch(&self.pool);
            while let Some(result) = rows.try_next().await? {
                let info_hash_data: &[u8] = result.get(structure.column_infohash.as_str());
                let info_hash: [u8; 20] = <[u8; 20]>::try_from(hex::decode(info_hash_data).unwrap()[0..20].as_ref()).unwrap();
                tracker.add_whitelist(InfoHash(info_hash));
                hashes += 1;
            }
            start += length;
            if hashes < start {
                break;
            }
            info!("[MySQL] Handled {hashes} whitelisted torrents");
        }
        info!("[MySQL] Handled {hashes} whitelisted torrents");
        Ok(hashes)
    }

    #[tracing::instrument(level = "debug")]
    pub async fn save_whitelist(&self, tracker: Arc<TorrentTracker>, whitelists: Vec<(InfoHash, UpdatesAction)>) -> Result<u64, Error>
    {
        let mut whitelist_transaction = self.pool.begin().await?;
        let mut whitelist_handled_entries = 0u64;
        let structure = tracker.config.deref().clone().database_structure.clone().whitelist;
        for (info_hash, updates_action) in whitelists.iter() {
            whitelist_handled_entries += 1;
            match updates_action {
                UpdatesAction::Remove => {
                    if tracker.config.deref().clone().database.remove_action {
                        let string_format = match tracker.config.deref().clone().database_structure.whitelist.bin_type_infohash {
                            true => {
                                format!(
                                    "DELETE FROM `{}` WHERE `{}`=UNHEX('{}')",
                                    structure.table_name,
                                    structure.column_infohash,
                                    info_hash
                                )
                            }
                            false => {
                                format!(
                                    "DELETE FROM `{}` WHERE `{}`='{}'",
                                    structure.table_name,
                                    structure.column_infohash,
                                    info_hash
                                )
                            }
                        };
                        match sqlx::query(string_format.as_str()).execute(&mut *whitelist_transaction).await {
                            Ok(_) => {}
                            Err(e) => {
                                error!("[MySQL] Error: {e}");
                                return Err(e);
                            }
                        }
                    }
                }
                UpdatesAction::Add | UpdatesAction::Update => {
                    let string_format = match tracker.config.deref().clone().database_structure.whitelist.bin_type_infohash {
                        true => {
                            format!(
                                "INSERT IGNORE INTO `{}` (`{}`) VALUES (UNHEX('{}'))",
                                structure.table_name,
                                structure.column_infohash,
                                info_hash
                            )
                        }
                        false => {
                            format!(
                                "INSERT IGNORE INTO `{}` (`{}`) VALUES ('{}')",
                                structure.table_name,
                                structure.column_infohash,
                                info_hash
                            )
                        }
                    };
                    match sqlx::query(string_format.as_str()).execute(&mut *whitelist_transaction).await {
                        Ok(_) => {}
                        Err(e) => {
                            error!("[MySQL] Error: {e}");
                            return Err(e);
                        }
                    }
                }
            }
            if (whitelist_handled_entries as f64 / 1000f64).fract() == 0.0 {
                info!("[MySQL] Handled {whitelist_handled_entries} whitelisted torrents");
            }
        }
        info!("[MySQL] Handled {whitelist_handled_entries} whitelisted torrents");
        let _ = self.commit(whitelist_transaction).await;
        Ok(whitelist_handled_entries)
    }

    #[tracing::instrument(level = "debug")]
    pub async fn load_blacklist(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error>
    {
        let mut start = 0u64;
        let length = 100000u64;
        let mut hashes = 0u64;
        let structure = tracker.config.deref().clone().database_structure.clone().blacklist;
        loop {
            let string_format = match tracker.config.deref().clone().database_structure.blacklist.bin_type_infohash {
                true => {
                    format!(
                        "SELECT HEX(`{}`) AS `{}` FROM `{}` LIMIT {}, {}",
                        structure.column_infohash,
                        structure.column_infohash,
                        structure.table_name,
                        start,
                        length
                    )
                }
                false => {
                    format!(
                        "SELECT `{}` FROM `{}` LIMIT {}, {}",
                        structure.column_infohash,
                        structure.table_name,
                        start,
                        length
                    )
                }
            };
            let mut rows = sqlx::query(string_format.as_str()).fetch(&self.pool);
            while let Some(result) = rows.try_next().await? {
                let info_hash_data: &[u8] = result.get(structure.column_infohash.as_str());
                let info_hash: [u8; 20] = <[u8; 20]>::try_from(hex::decode(info_hash_data).unwrap()[0..20].as_ref()).unwrap();
                tracker.add_blacklist(InfoHash(info_hash));
                hashes += 1;
            }
            start += length;
            if hashes < start {
                break;
            }
            info!("[MySQL] Handled {hashes} blacklisted torrents");
        }
        info!("[MySQL] Handled {hashes} blacklisted torrents");
        Ok(hashes)
    }

    #[tracing::instrument(level = "debug")]
    pub async fn save_blacklist(&self, tracker: Arc<TorrentTracker>, blacklists: Vec<(InfoHash, UpdatesAction)>) -> Result<u64, Error>
    {
        let mut blacklist_transaction = self.pool.begin().await?;
        let mut blacklist_handled_entries = 0u64;
        let structure = tracker.config.deref().clone().database_structure.clone().blacklist;
        for (info_hash, updates_action) in blacklists.iter() {
            blacklist_handled_entries += 1;
            match updates_action {
                UpdatesAction::Remove => {
                    if tracker.config.deref().clone().database.remove_action {
                        let string_format = match tracker.config.deref().clone().database_structure.blacklist.bin_type_infohash {
                            true => {
                                format!(
                                    "DELETE FROM `{}` WHERE `{}`=UNHEX('{}')",
                                    structure.table_name,
                                    structure.column_infohash,
                                    info_hash
                                )
                            }
                            false => {
                                format!(
                                    "DELETE FROM `{}` WHERE `{}`='{}'",
                                    structure.table_name,
                                    structure.column_infohash,
                                    info_hash
                                )
                            }
                        };
                        match sqlx::query(string_format.as_str()).execute(&mut *blacklist_transaction).await {
                            Ok(_) => {}
                            Err(e) => {
                                error!("[MySQL] Error: {e}");
                                return Err(e);
                            }
                        }
                    }
                }
                UpdatesAction::Add | UpdatesAction::Update => {
                    let string_format = match tracker.config.deref().clone().database_structure.blacklist.bin_type_infohash {
                        true => {
                            format!(
                                "INSERT IGNORE INTO `{}` (`{}`) VALUES (UNHEX('{}'))",
                                structure.table_name,
                                structure.column_infohash,
                                info_hash
                            )
                        }
                        false => {
                            format!(
                                "INSERT IGNORE INTO `{}` (`{}`) VALUES ('{}')",
                                structure.table_name,
                                structure.column_infohash,
                                info_hash
                            )
                        }
                    };
                    match sqlx::query(string_format.as_str()).execute(&mut *blacklist_transaction).await {
                        Ok(_) => {}
                        Err(e) => {
                            error!("[MySQL] Error: {e}");
                            return Err(e);
                        }
                    }
                }
            }
            if (blacklist_handled_entries as f64 / 1000f64).fract() == 0.0 {
                info!("[MySQL] Handled {blacklist_handled_entries} blacklisted torrents");
            }
        }
        info!("[MySQL] Handled {blacklist_handled_entries} blacklisted torrents");
        let _ = self.commit(blacklist_transaction).await;
        Ok(blacklist_handled_entries)
    }

    #[tracing::instrument(level = "debug")]
    pub async fn load_keys(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error>
    {
        let mut start = 0u64;
        let length = 100000u64;
        let mut hashes = 0u64;
        let structure = tracker.config.deref().clone().database_structure.clone().keys;
        loop {
            let string_format = match tracker.config.deref().clone().database_structure.keys.bin_type_hash {
                true => {
                    format!(
                        "SELECT HEX(`{}`) AS `{}`,`{}` FROM `{}` LIMIT {}, {}",
                        structure.column_hash,
                        structure.column_hash,
                        structure.column_timeout,
                        structure.table_name,
                        start,
                        length
                    )
                }
                false => {
                    format!(
                        "SELECT `{}`, `{}` FROM `{}` LIMIT {}, {}",
                        structure.column_hash,
                        structure.column_timeout,
                        structure.table_name,
                        start,
                        length
                    )
                }
            };
            let mut rows = sqlx::query(string_format.as_str()).fetch(&self.pool);
            while let Some(result) = rows.try_next().await? {
                let hash_data: &[u8] = result.get(structure.column_hash.as_str());
                let hash: [u8; 20] = <[u8; 20]>::try_from(hex::decode(hash_data).unwrap()[0..20].as_ref()).unwrap();
                let timeout: i64 = result.get(structure.column_timeout.as_str());
                tracker.add_key(InfoHash(hash), timeout);
                hashes += 1;
            }
            start += length;
            if hashes < start {
                break;
            }
            info!("[MySQL] Handled {hashes} keys");
        }
        info!("[MySQL] Handled {hashes} keys");
        Ok(hashes)
    }

    #[tracing::instrument(level = "debug")]
    pub async fn save_keys(&self, tracker: Arc<TorrentTracker>, keys: BTreeMap<InfoHash, (i64, UpdatesAction)>) -> Result<u64, Error>
    {
        let mut keys_transaction = self.pool.begin().await?;
        let mut keys_handled_entries = 0u64;
        let structure = tracker.config.deref().clone().database_structure.clone().keys;
        for (hash, (timeout, update_action)) in keys.iter() {
            keys_handled_entries += 1;
            match update_action {
                UpdatesAction::Remove => {
                    if tracker.config.deref().clone().database.remove_action {
                        let string_format = match tracker.config.deref().clone().database_structure.keys.bin_type_hash {
                            true => {
                                format!(
                                    "DELETE FROM `{}` WHERE `{}`=UNHEX('{}')",
                                    structure.table_name,
                                    structure.column_hash,
                                    hash
                                )
                            }
                            false => {
                                format!(
                                    "DELETE FROM `{}` WHERE `{}`='{}'",
                                    structure.table_name,
                                    structure.column_hash,
                                    hash
                                )
                            }
                        };
                        match sqlx::query(string_format.as_str()).execute(&mut *keys_transaction).await {
                            Ok(_) => {}
                            Err(e) => {
                                error!("[MySQL] Error: {e}");
                                return Err(e);
                            }
                        }
                    }
                }
                UpdatesAction::Add | UpdatesAction::Update => {
                    let string_format = match tracker.config.deref().clone().database_structure.keys.bin_type_hash {
                        true => {
                            format!(
                                "INSERT INTO `{}` (`{}`, `{}`) VALUES (UNHEX('{}'), {}) ON DUPLICATE KEY UPDATE `{}`=VALUES(`{}`), `{}`=VALUES(`{}`)",
                                structure.table_name,
                                structure.column_hash,
                                structure.column_timeout,
                                hash,
                                timeout,
                                structure.column_hash,
                                structure.column_hash,
                                structure.column_timeout,
                                structure.column_timeout
                            )
                        }
                        false => {
                            format!(
                                "INSERT INTO `{}` (`{}`, `{}`) VALUES ('{}', {}) ON DUPLICATE KEY UPDATE `{}`=VALUES(`{}`), `{}`=VALUES(`{}`)",
                                structure.table_name,
                                structure.column_hash,
                                structure.column_timeout,
                                hash,
                                timeout,
                                structure.column_hash,
                                structure.column_hash,
                                structure.column_timeout,
                                structure.column_timeout
                            )
                        }
                    };
                    match sqlx::query(string_format.as_str()).execute(&mut *keys_transaction).await {
                        Ok(_) => {}
                        Err(e) => {
                            error!("[MySQL] Error: {e}");
                            return Err(e);
                        }
                    }
                }
            }
            if (keys_handled_entries as f64 / 1000f64).fract() == 0.0 {
                info!("[MySQL] Handled {keys_handled_entries} keys");
            }
        }
        info!("[MySQL] Handled {keys_handled_entries} keys");
        let _ = self.commit(keys_transaction).await;
        Ok(keys_handled_entries)
    }

    #[tracing::instrument(level = "debug")]
    pub async fn load_users(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error>
    {
        let mut start = 0u64;
        let length = 100000u64;
        let mut hashes = 0u64;
        let structure = tracker.config.deref().clone().database_structure.clone().users;
        loop {
            let string_format = match tracker.config.deref().clone().database_structure.users.id_uuid {
                true => {
                    match tracker.config.deref().clone().database_structure.users.bin_type_key {
                        true => {
                            format!(
                                "SELECT `{}`, HEX(`{}`) AS `{}`, `{}`, `{}`, `{}`, `{}`, `{}` FROM `{}` LIMIT {}, {}",
                                structure.column_uuid,
                                structure.column_key,
                                structure.column_key,
                                structure.column_uploaded,
                                structure.column_downloaded,
                                structure.column_completed,
                                structure.column_updated,
                                structure.column_active,
                                structure.table_name,
                                start,
                                length
                            )
                        }
                        false => {
                            format!(
                                "SELECT `{}`, `{}`, `{}`, `{}`, `{}`, `{}`, `{}` FROM `{}` LIMIT {}, {}",
                                structure.column_uuid,
                                structure.column_key,
                                structure.column_uploaded,
                                structure.column_downloaded,
                                structure.column_completed,
                                structure.column_updated,
                                structure.column_active,
                                structure.table_name,
                                start,
                                length
                            )
                        }
                    }
                }
                false => {
                    match tracker.config.deref().clone().database_structure.users.bin_type_key {
                        true => {
                            format!(
                                "SELECT `{}`, HEX(`{}`) AS `{}`, `{}`, `{}`, `{}`, `{}`, `{}` FROM `{}` LIMIT {}, {}",
                                structure.column_id,
                                structure.column_key,
                                structure.column_key,
                                structure.column_uploaded,
                                structure.column_downloaded,
                                structure.column_completed,
                                structure.column_updated,
                                structure.column_active,
                                structure.table_name,
                                start,
                                length
                            )
                        }
                        false => {
                            format!(
                                "SELECT `{}`, `{}`, `{}`, `{}`, `{}`, `{}`, `{}` FROM `{}` LIMIT {}, {}",
                                structure.column_id,
                                structure.column_key,
                                structure.column_uploaded,
                                structure.column_downloaded,
                                structure.column_completed,
                                structure.column_updated,
                                structure.column_active,
                                structure.table_name,
                                start,
                                length
                            )
                        }
                    }
                }
            };
            let mut rows = sqlx::query(string_format.as_str()).fetch(&self.pool);
            while let Some(result) = rows.try_next().await? {
                let hash = match tracker.config.deref().clone().database_structure.users.id_uuid {
                    true => {
                        let uuid_data: &[u8] = result.get(structure.column_uuid.as_str());
                        let mut hasher = Sha1::new();
                        hasher.update(uuid_data);
                        
                        <[u8; 20]>::try_from(hasher.finalize().as_slice()).unwrap()
                    }
                    false => {
                        let id_data: &[u8] = result.get(structure.column_id.as_str());
                        let mut hasher = Sha1::new();
                        hasher.update(id_data);
                        
                        <[u8; 20]>::try_from(hasher.finalize().as_slice()).unwrap()
                    }
                };
                tracker.add_user(UserId(hash), UserEntryItem {
                    key: UserId::from_str(result.get(structure.column_key.as_str())).unwrap(),
                    user_id: match tracker.config.deref().clone().database_structure.users.id_uuid {
                        true => { None }
                        false => { Some(result.get(structure.column_id.as_str())) }
                    },
                    user_uuid: match tracker.config.deref().clone().database_structure.users.id_uuid {
                        true => { Some(result.get(structure.column_uuid.as_str())) }
                        false => { None }
                    },
                    uploaded: result.get::<i64, &str>(structure.column_uploaded.as_str()) as u64,
                    downloaded: result.get::<i64, &str>(structure.column_downloaded.as_str()) as u64,
                    completed: result.get::<i64, &str>(structure.column_completed.as_str()) as u64,
                    updated: result.get::<i32, &str>(structure.column_updated.as_str()) as u64,
                    active: result.get::<i8, &str>(structure.column_active.as_str()) as u8,
                    torrents_active: Default::default(),
                });
                hashes += 1;
            }
            start += length;
            if hashes < start {
                break;
            }
            info!("[MySQL] Loaded {hashes} users");
        }
        info!("[MySQL] Loaded {hashes} users");
        Ok(hashes)
    }

    #[tracing::instrument(level = "debug")]
    pub async fn save_users(&self, tracker: Arc<TorrentTracker>, users: BTreeMap<UserId, (UserEntryItem, UpdatesAction)>) -> Result<(), Error>
    {
        let mut users_transaction = self.pool.begin().await?;
        let mut users_handled_entries = 0u64;
        let structure = tracker.config.deref().clone().database_structure.clone().users;
        for (_, (user_entry_item, updates_action)) in users.iter() {
            users_handled_entries += 1;
            match updates_action {
                UpdatesAction::Remove => {
                    if tracker.config.deref().clone().database.remove_action {
                        let string_format = match tracker.config.deref().clone().database_structure.users.id_uuid {
                            true => {
                                format!(
                                    "DELETE FROM `{}` WHERE `{}`='{}'",
                                    structure.table_name,
                                    structure.column_uuid,
                                    user_entry_item.user_uuid.clone().unwrap()
                                )
                            }
                            false => {
                                format!(
                                    "DELETE FROM `{}` WHERE `{}`='{}'",
                                    structure.table_name,
                                    structure.column_id,
                                    user_entry_item.user_id.unwrap()
                                )
                            }
                        };
                        match sqlx::query(string_format.as_str()).execute(&mut *users_transaction).await {
                            Ok(_) => {}
                            Err(e) => {
                                error!("[MySQL] Error: {e}");
                                return Err(e);
                            }
                        }
                    }
                }
                UpdatesAction::Add | UpdatesAction::Update => {
                    let string_format = match  tracker.config.deref().clone().database.insert_vacant {
                        true => {
                            match tracker.config.deref().clone().database_structure.users.id_uuid {
                                true => {
                                    match tracker.config.deref().clone().database_structure.users.bin_type_key {
                                        true => {
                                            format!(
                                                "INSERT INTO `{}` (`{}`, `{}`, `{}`, `{}`, `{}`, `{}`, `{}`) VALUES ('{}', UNHEX('{}'), {}, {}, {}, {}, {}) ON DUPLICATE KEY UPDATE `{}`=VALUES(`{}`), `{}`=VALUES(`{}`), `{}`=VALUES(`{}`), `{}`=VALUES(`{}`), `{}`=VALUES(`{}`), `{}`=VALUES(`{}`)",
                                                structure.table_name,
                                                structure.column_uuid,
                                                structure.column_key,
                                                structure.column_uploaded,
                                                structure.column_downloaded,
                                                structure.column_completed,
                                                structure.column_active,
                                                structure.column_updated,
                                                user_entry_item.user_uuid.clone().unwrap(),
                                                user_entry_item.key,
                                                user_entry_item.uploaded,
                                                user_entry_item.downloaded,
                                                user_entry_item.completed,
                                                user_entry_item.active,
                                                user_entry_item.updated,
                                                structure.column_key,
                                                structure.column_key,
                                                structure.column_uploaded,
                                                structure.column_uploaded,
                                                structure.column_downloaded,
                                                structure.column_downloaded,
                                                structure.column_completed,
                                                structure.column_completed,
                                                structure.column_active,
                                                structure.column_active,
                                                structure.column_updated,
                                                structure.column_updated
                                            )
                                        }
                                        false => {
                                            format!(
                                                "INSERT INTO `{}` (`{}`, `{}`, `{}`, `{}`, `{}`, `{}`, `{}`) VALUES ('{}', '{}', {}, {}, {}, {}, {}) ON DUPLICATE KEY UPDATE `{}`=VALUES(`{}`), `{}`=VALUES(`{}`), `{}`=VALUES(`{}`), `{}`=VALUES(`{}`), `{}`=VALUES(`{}`), `{}`=VALUES(`{}`)",
                                                structure.table_name,
                                                structure.column_uuid,
                                                structure.column_key,
                                                structure.column_uploaded,
                                                structure.column_downloaded,
                                                structure.column_completed,
                                                structure.column_active,
                                                structure.column_updated,
                                                user_entry_item.user_uuid.clone().unwrap(),
                                                user_entry_item.key,
                                                user_entry_item.uploaded,
                                                user_entry_item.downloaded,
                                                user_entry_item.completed,
                                                user_entry_item.active,
                                                user_entry_item.updated,
                                                structure.column_key,
                                                structure.column_key,
                                                structure.column_uploaded,
                                                structure.column_uploaded,
                                                structure.column_downloaded,
                                                structure.column_downloaded,
                                                structure.column_completed,
                                                structure.column_completed,
                                                structure.column_active,
                                                structure.column_active,
                                                structure.column_updated,
                                                structure.column_updated
                                            )
                                        }
                                    }
                                }
                                false => {
                                    match tracker.config.deref().clone().database_structure.users.bin_type_key {
                                        true => {
                                            format!(
                                                "INSERT INTO `{}` (`{}`, `{}`, `{}`, `{}`, `{}`, `{}`, `{}`) VALUES ('{}', UNHEX('{}'), {}, {}, {}, {}, {}) ON DUPLICATE KEY UPDATE `{}`=VALUES(`{}`), `{}`=VALUES(`{}`), `{}`=VALUES(`{}`), `{}`=VALUES(`{}`), `{}`=VALUES(`{}`), `{}`=VALUES(`{}`)",
                                                structure.table_name,
                                                structure.column_id,
                                                structure.column_key,
                                                structure.column_uploaded,
                                                structure.column_downloaded,
                                                structure.column_completed,
                                                structure.column_active,
                                                structure.column_updated,
                                                user_entry_item.user_id.unwrap(),
                                                user_entry_item.key,
                                                user_entry_item.uploaded,
                                                user_entry_item.downloaded,
                                                user_entry_item.completed,
                                                user_entry_item.active,
                                                user_entry_item.updated,
                                                structure.column_key,
                                                structure.column_key,
                                                structure.column_uploaded,
                                                structure.column_uploaded,
                                                structure.column_downloaded,
                                                structure.column_downloaded,
                                                structure.column_completed,
                                                structure.column_completed,
                                                structure.column_active,
                                                structure.column_active,
                                                structure.column_updated,
                                                structure.column_updated
                                            )
                                        }
                                        false => {
                                            format!(
                                                "INSERT INTO `{}` (`{}`, `{}`, `{}`, `{}`, `{}`, `{}`, `{}`) VALUES ('{}', '{}', {}, {}, {}, {}, {}) ON DUPLICATE KEY UPDATE `{}`=VALUES(`{}`), `{}`=VALUES(`{}`), `{}`=VALUES(`{}`), `{}`=VALUES(`{}`), `{}`=VALUES(`{}`), `{}`=VALUES(`{}`)",
                                                structure.table_name,
                                                structure.column_id,
                                                structure.column_completed,
                                                structure.column_active,
                                                structure.column_downloaded,
                                                structure.column_key,
                                                structure.column_uploaded,
                                                structure.column_updated,
                                                user_entry_item.user_id.unwrap(),
                                                user_entry_item.completed,
                                                user_entry_item.active,
                                                user_entry_item.downloaded,
                                                user_entry_item.key,
                                                user_entry_item.uploaded,
                                                user_entry_item.updated,
                                                structure.column_completed,
                                                structure.column_completed,
                                                structure.column_active,
                                                structure.column_active,
                                                structure.column_downloaded,
                                                structure.column_downloaded,
                                                structure.column_key,
                                                structure.column_key,
                                                structure.column_uploaded,
                                                structure.column_uploaded,
                                                structure.column_updated,
                                                structure.column_updated
                                            )
                                        }
                                    }
                                }
                            }
                        }
                        false => {
                            match tracker.config.deref().clone().database_structure.users.id_uuid {
                                true => {
                                    match tracker.config.deref().clone().database_structure.users.bin_type_key {
                                        true => {
                                            format!(
                                                "UPDATE IGNORE `{}` SET `{}`={}, `{}`={}, `{}`={}, `{}`=UNHEX('{}'), `{}`={}, `{}`={} WHERE `{}`=UNHEX('{}')",
                                                structure.table_name,
                                                structure.column_completed,
                                                user_entry_item.completed,
                                                structure.column_active,
                                                user_entry_item.active,
                                                structure.column_downloaded,
                                                user_entry_item.downloaded,
                                                structure.column_key,
                                                user_entry_item.key,
                                                structure.column_uploaded,
                                                user_entry_item.uploaded,
                                                structure.column_updated,
                                                user_entry_item.updated,
                                                structure.column_uuid,
                                                user_entry_item.user_uuid.clone().unwrap(),
                                            )
                                        }
                                        false => {
                                            format!(
                                                "UPDATE IGNORE `{}` SET `{}`={}, `{}`={}, `{}`={}, `{}`='{}', `{}`={}, `{}`={} WHERE `{}`='{}'",
                                                structure.table_name,
                                                structure.column_completed,
                                                user_entry_item.completed,
                                                structure.column_active,
                                                user_entry_item.active,
                                                structure.column_downloaded,
                                                user_entry_item.downloaded,
                                                structure.column_key,
                                                user_entry_item.key,
                                                structure.column_uploaded,
                                                user_entry_item.uploaded,
                                                structure.column_updated,
                                                user_entry_item.updated,
                                                structure.column_uuid,
                                                user_entry_item.user_uuid.clone().unwrap(),
                                            )
                                        }
                                    }
                                }
                                false => {
                                    match tracker.config.deref().clone().database_structure.users.bin_type_key {
                                        true => {
                                            format!(
                                                "UPDATE IGNORE `{}` SET `{}`={}, `{}`={}, `{}`={}, `{}`=UNHEX('{}'), `{}`={}, `{}`={} WHERE `{}`=UNHEX('{}')",
                                                structure.table_name,
                                                structure.column_completed,
                                                user_entry_item.completed,
                                                structure.column_active,
                                                user_entry_item.active,
                                                structure.column_downloaded,
                                                user_entry_item.downloaded,
                                                structure.column_key,
                                                user_entry_item.key,
                                                structure.column_uploaded,
                                                user_entry_item.uploaded,
                                                structure.column_updated,
                                                user_entry_item.updated,
                                                structure.column_id,
                                                user_entry_item.user_id.unwrap(),
                                            )
                                        }
                                        false => {
                                            format!(
                                                "UPDATE IGNORE `{}` SET `{}`={}, `{}`={}, `{}`={}, `{}`='{}', `{}`={}, `{}`={} WHERE `{}`='{}'",
                                                structure.table_name,
                                                structure.column_completed,
                                                user_entry_item.completed,
                                                structure.column_active,
                                                user_entry_item.active,
                                                structure.column_downloaded,
                                                user_entry_item.downloaded,
                                                structure.column_key,
                                                user_entry_item.key,
                                                structure.column_uploaded,
                                                user_entry_item.uploaded,
                                                structure.column_updated,
                                                user_entry_item.updated,
                                                structure.column_id,
                                                user_entry_item.user_id.unwrap(),
                                            )
                                        }
                                    }
                                }
                            }
                        }
                    };
                    match sqlx::query(string_format.as_str()).execute(&mut *users_transaction).await {
                        Ok(_) => {}
                        Err(e) => {
                            error!("[MySQL] Error: {e}");
                            return Err(e);
                        }
                    }
                }
            }
            if (users_handled_entries as f64 / 1000f64).fract() == 0.0 || users.len() as u64 == users_handled_entries {
                info!("[MySQL] Handled {users_handled_entries} users");
            }
        }
        info!("[MySQL] Handled {users_handled_entries} users");
        self.commit(users_transaction).await
    }

    #[tracing::instrument(level = "debug")]
    pub async fn reset_seeds_peers(&self, tracker: Arc<TorrentTracker>) -> Result<(), Error>
    {
        let mut reset_seeds_peers_transaction = self.pool.begin().await?;
        let structure = tracker.config.deref().clone().database_structure.clone().torrents;
        let string_format = format!(
            "UPDATE `{}` SET `{}`=0, `{}`=0",
            structure.table_name,
            structure.column_seeds,
            structure.column_peers
        );
        match sqlx::query(string_format.as_str()).execute(&mut *reset_seeds_peers_transaction).await {
            Ok(_) => {}
            Err(e) => {
                error!("[MySQL] Error: {e}");
                return Err(e);
            }
        }
        let _ = self.commit(reset_seeds_peers_transaction).await;
        Ok(())
    }

    pub async fn commit(&self, transaction: Transaction<'_, MySql>) -> Result<(), Error>
    {
        match transaction.commit().await {
            Ok(_) => {
                Ok(())
            }
            Err(e) => {
                error!("[MySQL] Error: {e}");
                Err(e)
            }
        }
    }
}