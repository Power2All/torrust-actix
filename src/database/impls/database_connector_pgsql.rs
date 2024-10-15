use std::collections::BTreeMap;
use std::ops::Deref;
use std::process::exit;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use async_std::task;
use futures_util::TryStreamExt;
use log::{error, info};
use sha1::{Digest, Sha1};
use sqlx::{ConnectOptions, Error, Pool, Postgres, Row, Transaction};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use crate::config::structs::configuration::Configuration;
use crate::database::enums::database_drivers::DatabaseDrivers;
use crate::database::structs::database_connector::DatabaseConnector;
use crate::database::structs::database_connector_pgsql::DatabaseConnectorPgSQL;
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::tracker::structs::user_entry_item::UserEntryItem;
use crate::tracker::structs::user_id::UserId;

impl DatabaseConnectorPgSQL {
    pub async fn create(dsl: &str) -> Result<Pool<Postgres>, Error>
    {
        let options = PgConnectOptions::from_str(dsl)?
            .log_statements(log::LevelFilter::Debug)
            .log_slow_statements(log::LevelFilter::Debug, Duration::from_secs(1));
        PgPoolOptions::new().connect_with(options).await
    }

    pub async fn database_connector(config: Arc<Configuration>, create_database: bool) -> DatabaseConnector
    {
        let pgsql_connect = DatabaseConnectorPgSQL::create(config.database.clone().unwrap().path.unwrap().as_str()).await;
        if pgsql_connect.is_err() {
            error!("[PgSQL] Unable to connect to PgSQL on DSL {}", config.database.clone().unwrap().path.unwrap());
            error!("[PgSQL] Message: {:#?}", pgsql_connect.unwrap_err().into_database_error().unwrap().message());
            exit(1);
        }

        let mut structure = DatabaseConnector { mysql: None, sqlite: None, pgsql: None, engine: None };
        structure.pgsql = Some(DatabaseConnectorPgSQL { pool: pgsql_connect.unwrap() });
        structure.engine = Some(DatabaseDrivers::pgsql);

        if create_database {
            let pool = &structure.pgsql.clone().unwrap().pool;
            info!("[BOOT] Database creation triggered for PgSQL.");

            // Create Torrent DB
            info!("[BOOT PgSQL] Creating table {}", config.database_structure.clone().unwrap().torrents.unwrap().database_name);
            match config.database_structure.clone().unwrap().torrents.unwrap().bin_type_infohash {
                true => {
                    match sqlx::query(
                        format!(
                            "CREATE TABLE IF NOT EXISTS public.{} ({} bytea NOT NULL, {} integer NOT NULL DEFAULT 0, {} integer NOT NULL DEFAULT 0, {} bigint NOT NULL DEFAULT 0, CONSTRAINT torrents_pkey PRIMARY KEY ({})) TABLESPACE pg_default",
                            config.database_structure.clone().unwrap().torrents.unwrap().database_name,
                            config.database_structure.clone().unwrap().torrents.unwrap().column_infohash,
                            config.database_structure.clone().unwrap().torrents.unwrap().column_seeds,
                            config.database_structure.clone().unwrap().torrents.unwrap().column_peers,
                            config.database_structure.clone().unwrap().torrents.unwrap().column_completed,
                            config.database_structure.clone().unwrap().torrents.unwrap().column_infohash
                        ).as_str()
                    ).execute(pool).await {
                        Ok(_) => {}
                        Err(error) => { panic!("[PgSQL] Error: {}", error); }
                    }
                }
                false => {
                    match sqlx::query(
                        format!(
                            "CREATE TABLE IF NOT EXISTS public.{} ({} character(40) NOT NULL, {} integer NOT NULL DEFAULT 0, {} integer NOT NULL DEFAULT 0, {} bigint NOT NULL DEFAULT 0, CONSTRAINT torrents_pkey PRIMARY KEY ({})) TABLESPACE pg_default",
                            config.database_structure.clone().unwrap().torrents.unwrap().database_name,
                            config.database_structure.clone().unwrap().torrents.unwrap().column_infohash,
                            config.database_structure.clone().unwrap().torrents.unwrap().column_seeds,
                            config.database_structure.clone().unwrap().torrents.unwrap().column_peers,
                            config.database_structure.clone().unwrap().torrents.unwrap().column_completed,
                            config.database_structure.clone().unwrap().torrents.unwrap().column_infohash
                        ).as_str()
                    ).execute(pool).await {
                        Ok(_) => {}
                        Err(error) => { panic!("[PgSQL] Error: {}", error); }
                    }
                }
            }

            // Create Whitelist DB
            info!("[BOOT PgSQL] Creating table {}", config.database_structure.clone().unwrap().whitelist.unwrap().database_name);
            match config.database_structure.clone().unwrap().whitelist.unwrap().bin_type_infohash {
                true => {
                    match sqlx::query(
                        format!(
                            "CREATE TABLE IF NOT EXISTS public.{} ({} bytea NOT NULL, CONSTRAINT whitelist_pkey PRIMARY KEY ({})) TABLESPACE pg_default",
                            config.database_structure.clone().unwrap().whitelist.unwrap().database_name,
                            config.database_structure.clone().unwrap().whitelist.unwrap().column_infohash,
                            config.database_structure.clone().unwrap().whitelist.unwrap().column_infohash
                        ).as_str()
                    ).execute(pool).await {
                        Ok(_) => {}
                        Err(error) => { panic!("[PgSQL] Error: {}", error); }
                    }
                }
                false => {
                    match sqlx::query(
                        format!(
                            "CREATE TABLE IF NOT EXISTS public.{} ({} character(40) NOT NULL, CONSTRAINT whitelist_pkey PRIMARY KEY ({})) TABLESPACE pg_default",
                            config.database_structure.clone().unwrap().whitelist.unwrap().database_name,
                            config.database_structure.clone().unwrap().whitelist.unwrap().column_infohash,
                            config.database_structure.clone().unwrap().whitelist.unwrap().column_infohash
                        ).as_str()
                    ).execute(pool).await {
                        Ok(_) => {}
                        Err(error) => { panic!("[PgSQL] Error: {}", error); }
                    }
                }
            }

            // Create Blacklist DB
            info!("[BOOT PgSQL] Creating table {}", config.database_structure.clone().unwrap().blacklist.unwrap().database_name);
            match config.database_structure.clone().unwrap().blacklist.unwrap().bin_type_infohash {
                true => {
                    match sqlx::query(
                        format!(
                            "CREATE TABLE IF NOT EXISTS public.{} ({} bytea NOT NULL, CONSTRAINT blacklist_pkey PRIMARY KEY ({})) TABLESPACE pg_default",
                            config.database_structure.clone().unwrap().blacklist.unwrap().database_name,
                            config.database_structure.clone().unwrap().blacklist.unwrap().column_infohash,
                            config.database_structure.clone().unwrap().blacklist.unwrap().column_infohash
                        ).as_str()
                    ).execute(pool).await {
                        Ok(_) => {}
                        Err(error) => { panic!("[PgSQL] Error: {}", error); }
                    }
                }
                false => {
                    match sqlx::query(
                        format!(
                            "CREATE TABLE IF NOT EXISTS public.{} ({} character(40) NOT NULL, CONSTRAINT blacklist_pkey PRIMARY KEY ({})) TABLESPACE pg_default",
                            config.database_structure.clone().unwrap().blacklist.unwrap().database_name,
                            config.database_structure.clone().unwrap().blacklist.unwrap().column_infohash,
                            config.database_structure.clone().unwrap().blacklist.unwrap().column_infohash
                        ).as_str()
                    ).execute(pool).await {
                        Ok(_) => {}
                        Err(error) => { panic!("[PgSQL] Error: {}", error); }
                    }
                }
            }

            // Create Keys DB
            info!("[BOOT PgSQL] Creating table {}", config.database_structure.clone().unwrap().keys.unwrap().database_name);
            match config.database_structure.clone().unwrap().keys.unwrap().bin_type_hash {
                true => {
                    match sqlx::query(
                        format!(
                            "CREATE TABLE IF NOT EXISTS public.{} ({} bytea NOT NULL, {} integer NOT NULL DEFAULT 0, CONSTRAINT keys_pkey PRIMARY KEY ({})) TABLESPACE pg_default",
                            config.database_structure.clone().unwrap().keys.unwrap().database_name,
                            config.database_structure.clone().unwrap().keys.unwrap().column_hash,
                            config.database_structure.clone().unwrap().keys.unwrap().column_timeout,
                            config.database_structure.clone().unwrap().keys.unwrap().column_hash
                        ).as_str()
                    ).execute(pool).await {
                        Ok(_) => {}
                        Err(error) => { panic!("[PgSQL] Error: {}", error); }
                    }
                }
                false => {
                    match sqlx::query(
                        format!(
                            "CREATE TABLE IF NOT EXISTS public.{} ({} character(40) NOT NULL, {} integer NOT NULL DEFAULT 0, CONSTRAINT keys_pkey PRIMARY KEY ({})) TABLESPACE pg_default",
                            config.database_structure.clone().unwrap().keys.unwrap().database_name,
                            config.database_structure.clone().unwrap().keys.unwrap().column_hash,
                            config.database_structure.clone().unwrap().keys.unwrap().column_timeout,
                            config.database_structure.clone().unwrap().keys.unwrap().column_hash
                        ).as_str()
                    ).execute(pool).await {
                        Ok(_) => {}
                        Err(error) => { panic!("[PgSQL] Error: {}", error); }
                    }
                }
            }

            // Create Users DB
            info!("[BOOT PgSQL] Creating table {}", config.database_structure.clone().unwrap().users.unwrap().database_name);
            match config.database_structure.clone().unwrap().users.unwrap().id_uuid {
                true => {
                    match config.database_structure.clone().unwrap().users.unwrap().bin_type_key {
                        true => {
                            match sqlx::query(
                                format!(
                                    "CREATE TABLE IF NOT EXISTS public.{} ({} character(36) NOT NULL, {} bytea NOT NULL, {} bigint NOT NULL DEFAULT 0, {} bigint NOT NULL DEFAULT 0, {} bigint NOT NULL DEFAULT 0, {} smallint NOT NULL DEFAULT 0, {} integer NOT NULL DEFAULT 0, CONSTRAINT uuid_pkey PRIMARY KEY ({})) TABLESPACE pg_default",
                                    config.database_structure.clone().unwrap().users.unwrap().database_name,
                                    config.database_structure.clone().unwrap().users.unwrap().column_uuid,
                                    config.database_structure.clone().unwrap().users.unwrap().column_key,
                                    config.database_structure.clone().unwrap().users.unwrap().column_uploaded,
                                    config.database_structure.clone().unwrap().users.unwrap().column_downloaded,
                                    config.database_structure.clone().unwrap().users.unwrap().column_completed,
                                    config.database_structure.clone().unwrap().users.unwrap().column_active,
                                    config.database_structure.clone().unwrap().users.unwrap().column_updated,
                                    config.database_structure.clone().unwrap().users.unwrap().column_uuid
                                ).as_str()
                            ).execute(pool).await {
                                Ok(_) => {}
                                Err(error) => { panic!("[PgSQL] Error: {}", error); }
                            }
                        }
                        false => {
                            match sqlx::query(
                                format!(
                                    "CREATE TABLE IF NOT EXISTS public.{} ({} character(36) NOT NULL, {} character(40) NOT NULL, {} bigint NOT NULL DEFAULT 0, {} bigint NOT NULL DEFAULT 0, {} bigint NOT NULL DEFAULT 0, {} smallint NOT NULL DEFAULT 0, {} integer NOT NULL DEFAULT 0, CONSTRAINT uuid_pkey PRIMARY KEY ({})) TABLESPACE pg_default",
                                    config.database_structure.clone().unwrap().users.unwrap().database_name,
                                    config.database_structure.clone().unwrap().users.unwrap().column_uuid,
                                    config.database_structure.clone().unwrap().users.unwrap().column_key,
                                    config.database_structure.clone().unwrap().users.unwrap().column_uploaded,
                                    config.database_structure.clone().unwrap().users.unwrap().column_downloaded,
                                    config.database_structure.clone().unwrap().users.unwrap().column_completed,
                                    config.database_structure.clone().unwrap().users.unwrap().column_active,
                                    config.database_structure.clone().unwrap().users.unwrap().column_updated,
                                    config.database_structure.clone().unwrap().users.unwrap().column_uuid
                                ).as_str()
                            ).execute(pool).await {
                                Ok(_) => {}
                                Err(error) => { panic!("[PgSQL] Error: {}", error); }
                            }
                        }
                    }
                }
                false => {
                    match config.database_structure.clone().unwrap().users.unwrap().bin_type_key {
                        true => {
                            match sqlx::query(
                                format!(
                                    "CREATE TABLE IF NOT EXISTS public.{} ({} bigserial NOT NULL, {} bytea NOT NULL, {} bigint NOT NULL DEFAULT 0, {} bigint NOT NULL DEFAULT 0, {} bigint NOT NULL DEFAULT 0, {} smallint NOT NULL DEFAULT 0, {} integer NOT NULL DEFAULT 0, CONSTRAINT id_pkey PRIMARY KEY ({})) TABLESPACE pg_default",
                                    config.database_structure.clone().unwrap().users.unwrap().database_name,
                                    config.database_structure.clone().unwrap().users.unwrap().column_id,
                                    config.database_structure.clone().unwrap().users.unwrap().column_key,
                                    config.database_structure.clone().unwrap().users.unwrap().column_uploaded,
                                    config.database_structure.clone().unwrap().users.unwrap().column_downloaded,
                                    config.database_structure.clone().unwrap().users.unwrap().column_completed,
                                    config.database_structure.clone().unwrap().users.unwrap().column_active,
                                    config.database_structure.clone().unwrap().users.unwrap().column_updated,
                                    config.database_structure.clone().unwrap().users.unwrap().column_id
                                ).as_str()
                            ).execute(pool).await {
                                Ok(_) => {}
                                Err(error) => { panic!("[PgSQL] Error: {}", error); }
                            }
                        }
                        false => {
                            match sqlx::query(
                                format!(
                                    "CREATE TABLE IF NOT EXISTS public.{} ({} bigserial NOT NULL, {} character(40) NOT NULL, {} bigint NOT NULL DEFAULT 0, {} bigint NOT NULL DEFAULT 0, {} bigint NOT NULL DEFAULT 0, {} smallint NOT NULL DEFAULT 0, {} integer NOT NULL DEFAULT 0, CONSTRAINT id_pkey PRIMARY KEY ({})) TABLESPACE pg_default",
                                    config.database_structure.clone().unwrap().users.unwrap().database_name,
                                    config.database_structure.clone().unwrap().users.unwrap().column_id,
                                    config.database_structure.clone().unwrap().users.unwrap().column_key,
                                    config.database_structure.clone().unwrap().users.unwrap().column_uploaded,
                                    config.database_structure.clone().unwrap().users.unwrap().column_downloaded,
                                    config.database_structure.clone().unwrap().users.unwrap().column_completed,
                                    config.database_structure.clone().unwrap().users.unwrap().column_active,
                                    config.database_structure.clone().unwrap().users.unwrap().column_updated,
                                    config.database_structure.clone().unwrap().users.unwrap().column_id
                                ).as_str()
                            ).execute(pool).await {
                                Ok(_) => {}
                                Err(error) => { panic!("[PgSQL] Error: {}", error); }
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

    pub async fn load_torrents(&self, tracker: Arc<TorrentTracker>) -> Result<(u64, u64), Error>
    {
        let mut start = 0u64;
        let length = 100000u64;
        let mut torrents = 0u64;
        let mut completed = 0u64;
        let structure = match tracker.config.deref().clone().database_structure.clone().unwrap().torrents {
            None => { return Err(Error::RowNotFound); }
            Some(db_structure) => { db_structure }
        };
        loop {
            info!(
                "[PgSQL] Trying to querying {} torrents - Skip: {}",
                length,
                start
            );
            let string_format = match tracker.config.deref().clone().database_structure.unwrap().torrents.unwrap().bin_type_infohash {
                true => {
                    format!(
                        "SELECT encode({}::bytea, 'hex'), {} FROM {} LIMIT {}, {}",
                        structure.column_infohash,
                        structure.column_completed,
                        structure.database_name,
                        start,
                        length
                    )
                }
                false => {
                    format!(
                        "SELECT {}, {} FROM {} LIMIT {}, {}",
                        structure.column_infohash,
                        structure.column_completed,
                        structure.database_name,
                        start,
                        length
                    )
                }
            };
            let mut rows = sqlx::query(string_format.as_str()).fetch(&self.pool);
            while let Some(result) = rows.try_next().await? {
                let info_hash_data: &[u8] = result.get(structure.column_infohash.as_str());
                let info_hash: [u8; 20] = <[u8; 20]>::try_from(hex::decode(info_hash_data).unwrap()[0..20].as_ref()).unwrap();
                let completed_count: i64 = result.get(structure.column_completed.as_str());
                tracker.add_torrent(
                    InfoHash(info_hash),
                    TorrentEntry {
                        seeds: BTreeMap::new(),
                        peers: BTreeMap::new(),
                        completed: completed_count as u64,
                        updated: std::time::Instant::now()
                    }
                );
                torrents += 1;
                completed += completed_count as u64;
            }
            start += length;
            if torrents < start {
                break;
            }
        }
        tracker.set_stats(StatsEvent::Completed, completed as i64);
        Ok((torrents, completed))
    }

    pub async fn save_torrents(&self, tracker: Arc<TorrentTracker>, torrents: BTreeMap<InfoHash, TorrentEntry>) -> Result<(), Error>
    {
        let mut torrents_transaction = self.pool.begin().await?;
        let mut torrents_handled_entries = 0u64;
        let structure = match tracker.config.deref().clone().database_structure.clone().unwrap().torrents {
            None => { return Err(Error::RowNotFound); }
            Some(db_structure) => { db_structure }
        };
        for (info_hash, torrent_entry) in torrents.iter() {
            torrents_handled_entries += 1;
            match tracker.config.deref().clone().database.unwrap().insert_vacant {
                true => {
                    if tracker.config.deref().clone().database.unwrap().update_peers {
                        let string_format = match tracker.config.deref().clone().database_structure.unwrap().torrents.unwrap().bin_type_infohash {
                            true => {
                                format!(
                                    "INSERT INTO {} ({}, {}, {}) VALUES (decode('{}', 'hex'), {}, {}) ON CONFLICT ({}) DO UPDATE SET {}=excluded.{}, {}=excluded.{}",
                                    structure.database_name,
                                    structure.column_infohash,
                                    structure.column_seeds,
                                    structure.column_peers,
                                    info_hash,
                                    torrent_entry.seeds.len(),
                                    torrent_entry.peers.len(),
                                    structure.column_infohash,
                                    structure.column_seeds,
                                    structure.column_seeds,
                                    structure.column_peers,
                                    structure.column_peers
                                )
                            }
                            false => {
                                format!(
                                    "INSERT INTO {} ({}, {}, {}) VALUES ('{}', {}, {}) ON CONFLICT ({}) DO UPDATE SET {}=excluded.{}, {}=excluded.{}",
                                    structure.database_name,
                                    structure.column_infohash,
                                    structure.column_seeds,
                                    structure.column_peers,
                                    info_hash,
                                    torrent_entry.seeds.len(),
                                    torrent_entry.peers.len(),
                                    structure.column_infohash,
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
                                error!("[PgSQL] Error: {}", e.to_string());
                                return Err(e);
                            }
                        }
                    }
                    if tracker.config.deref().clone().database.unwrap().update_completed {
                        let string_format = match tracker.config.deref().clone().database_structure.unwrap().torrents.unwrap().bin_type_infohash {
                            true => {
                                format!(
                                    "INSERT INTO {} ({}, {}) VALUES (decode('{}', 'hex'), {}) ON CONFLICT ({}) DO UPDATE SET {}=excluded.{}",
                                    structure.database_name,
                                    structure.column_infohash,
                                    structure.column_completed,
                                    info_hash,
                                    torrent_entry.completed,
                                    structure.column_infohash,
                                    structure.column_completed,
                                    structure.column_completed
                                )
                            }
                            false => {
                                format!(
                                    "INSERT INTO {} ({}, {}) VALUES ('{}', {}) ON CONFLICT ({}) DO UPDATE SET {}=excluded.{}",
                                    structure.database_name,
                                    structure.column_infohash,
                                    structure.column_completed,
                                    info_hash,
                                    torrent_entry.completed,
                                    structure.column_infohash,
                                    structure.column_completed,
                                    structure.column_completed
                                )
                            }
                        };
                        match sqlx::query(string_format.as_str()).execute(&mut *torrents_transaction).await {
                            Ok(_) => {}
                            Err(e) => {
                                error!("[PgSQL] Error: {}", e.to_string());
                                return Err(e);
                            }
                        }
                    }
                }
                false => {
                    if tracker.config.deref().clone().database.unwrap().update_peers {
                        let string_format = match tracker.config.deref().clone().database_structure.unwrap().torrents.unwrap().bin_type_infohash {
                            true => {
                                format!(
                                    "UPDATE {} SET ({}, {}) = ({}, {}) WHERE {}=decode('{}', 'hex') AND NOT EXISTS (SELECT 1 FROM {} WHERE {}=decode('{}', 'hex'))",
                                    structure.database_name,
                                    structure.column_seeds,
                                    structure.column_peers,
                                    torrent_entry.seeds.len(),
                                    torrent_entry.peers.len(),
                                    structure.column_infohash,
                                    info_hash,
                                    structure.database_name,
                                    structure.column_infohash,
                                    info_hash
                                )
                            }
                            false => {
                                format!(
                                    "UPDATE {} SET ({}, {}) = ({}, {}) WHERE {}='{}' AND NOT EXISTS (SELECT 1 FROM {} WHERE {}='{}')",
                                    structure.database_name,
                                    structure.column_seeds,
                                    structure.column_peers,
                                    torrent_entry.seeds.len(),
                                    torrent_entry.peers.len(),
                                    structure.column_infohash,
                                    info_hash,
                                    structure.database_name,
                                    structure.column_infohash,
                                    info_hash
                                )
                            }
                        };
                        match sqlx::query(string_format.as_str()).execute(&mut *torrents_transaction).await {
                            Ok(_) => {}
                            Err(e) => {
                                error!("[PgSQL] Error: {}", e.to_string());
                                return Err(e);
                            }
                        }
                    }
                    if tracker.config.deref().clone().database.unwrap().update_completed {
                        let string_format = match tracker.config.deref().clone().database_structure.unwrap().torrents.unwrap().bin_type_infohash {
                            true => {
                                format!(
                                    "UPDATE {} SET {}={} WHERE {}=decode('{}', 'hex') AND EXISTS (SELECT 1 FROM {} WHERE {}=decode('{}', 'hex'))",
                                    structure.database_name,
                                    structure.column_completed,
                                    torrent_entry.completed,
                                    structure.column_infohash,
                                    info_hash,
                                    structure.database_name,
                                    structure.column_infohash,
                                    info_hash
                                )
                            }
                            false => {
                                format!(
                                    "UPDATE {} SET {}={} WHERE {}='{}' AND EXISTS (SELECT 1 FROM {} WHERE {}='{}')",
                                    structure.database_name,
                                    structure.column_completed,
                                    torrent_entry.completed,
                                    structure.column_infohash,
                                    info_hash,
                                    structure.database_name,
                                    structure.column_infohash,
                                    info_hash
                                )
                            }
                        };
                        match sqlx::query(string_format.as_str()).execute(&mut *torrents_transaction).await {
                            Ok(_) => {}
                            Err(e) => {
                                error!("[PgSQL] Error: {}", e.to_string());
                                return Err(e);
                            }
                        }
                    }
                }
            }
            if (torrents_handled_entries as f64 / 1000f64).fract() == 0.0 || torrents.len() as u64 == torrents_handled_entries {
                info!("[PgSQL] Handled {} torrents", torrents_handled_entries);
            }
        }
        self.commit(torrents_transaction).await
    }

    pub async fn load_whitelist(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error>
    {
        let mut start = 0u64;
        let length = 100000u64;
        let mut hashes = 0u64;
        let structure = match tracker.config.deref().clone().database_structure.clone().unwrap().whitelist {
            None => { return Err(Error::RowNotFound); }
            Some(db_structure) => { db_structure }
        };
        loop {
            info!(
                "[PgSQL] Trying to querying {} whitelisted hashes - Skip: {}",
                length,
                start
            );
            let string_format = match tracker.config.deref().clone().database_structure.unwrap().whitelist.unwrap().bin_type_infohash {
                true => {
                    format!(
                        "SELECT encode({}::bytea, 'hex') FROM {} LIMIT {}, {}",
                        structure.column_infohash,
                        structure.database_name,
                        start,
                        length
                    )
                }
                false => {
                    format!(
                        "SELECT {} FROM {} LIMIT {}, {}",
                        structure.column_infohash,
                        structure.database_name,
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
        }
        Ok(hashes)
    }

    pub async fn save_whitelist(&self, tracker: Arc<TorrentTracker>, whitelists: Vec<InfoHash>) -> Result<u64, Error>
    {
        let mut whitelist_transaction = self.pool.begin().await?;
        let mut whitelist_handled_entries = 0u64;
        let structure = match tracker.config.deref().clone().database_structure.clone().unwrap().whitelist {
            None => { return Err(Error::RowNotFound); }
            Some(db_structure) => { db_structure }
        };
        for info_hash in whitelists.iter() {
            whitelist_handled_entries += 1;
            let string_format = match tracker.config.deref().clone().database_structure.unwrap().whitelist.unwrap().bin_type_infohash {
                true => {
                    format!(
                        "INSERT INTO {} ({}) VALUES (decode('{}', 'hex')) ON CONFLICT DO NOTHING",
                        structure.database_name,
                        structure.column_infohash,
                        info_hash
                    )
                }
                false => {
                    format!(
                        "INSERT INTO {} ({}) VALUES ('{}') ON CONFLICT DO NOTHING",
                        structure.database_name,
                        structure.column_infohash,
                        info_hash
                    )
                }
            };
            match sqlx::query(string_format.as_str()).execute(&mut *whitelist_transaction).await {
                Ok(_) => {}
                Err(e) => {
                    error!("[PgSQL] Error: {}", e.to_string());
                    return Err(e);
                }
            }
            if (whitelist_handled_entries as f64 / 1000f64).fract() == 0.0 {
                info!("[PgSQL] Handled {} torrents", whitelist_handled_entries);
            }
        }
        info!("[PgSQL] Saved {} whitelisted torrents", whitelist_handled_entries);
        let _ = self.commit(whitelist_transaction).await;
        Ok(whitelist_handled_entries)
    }

    pub async fn load_blacklist(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error>
    {
        let mut start = 0u64;
        let length = 100000u64;
        let mut hashes = 0u64;
        let structure = match tracker.config.deref().clone().database_structure.clone().unwrap().blacklist {
            None => { return Err(Error::RowNotFound); }
            Some(db_structure) => { db_structure }
        };
        loop {
            info!(
                "[PgSQL] Trying to querying {} blacklisted hashes - Skip: {}",
                length,
                start
            );
            let string_format = match tracker.config.deref().clone().database_structure.unwrap().blacklist.unwrap().bin_type_infohash {
                true => {
                    format!(
                        "SELECT encode({}::bytea, 'hex') FROM {} LIMIT {}, {}",
                        structure.column_infohash,
                        structure.database_name,
                        start,
                        length
                    )
                }
                false => {
                    format!(
                        "SELECT {} FROM {} LIMIT {}, {}",
                        structure.column_infohash,
                        structure.database_name,
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
        }
        Ok(hashes)
    }

    pub async fn save_blacklist(&self, tracker: Arc<TorrentTracker>, blacklists: Vec<InfoHash>) -> Result<u64, Error>
    {
        let mut blacklist_transaction = self.pool.begin().await?;
        let mut blacklist_handled_entries = 0u64;
        let structure = match tracker.config.deref().clone().database_structure.clone().unwrap().blacklist {
            None => { return Err(Error::RowNotFound); }
            Some(db_structure) => { db_structure }
        };
        for info_hash in blacklists.iter() {
            blacklist_handled_entries += 1;
            let string_format = match tracker.config.deref().clone().database_structure.unwrap().blacklist.unwrap().bin_type_infohash {
                true => {
                    format!(
                        "INSERT INTO {} ({}) VALUES (decode('{}', 'hex')) ON CONFLICT DO NOTHING",
                        structure.database_name,
                        structure.column_infohash,
                        info_hash
                    )
                }
                false => {
                    format!(
                        "INSERT INTO {} ({}) VALUES ('{}') ON CONFLICT DO NOTHING",
                        structure.database_name,
                        structure.column_infohash,
                        info_hash
                    )
                }
            };
            match sqlx::query(string_format.as_str()).execute(&mut *blacklist_transaction).await {
                Ok(_) => {}
                Err(e) => {
                    error!("[PgSQL] Error: {}", e.to_string());
                    return Err(e);
                }
            }
            if (blacklist_handled_entries as f64 / 1000f64).fract() == 0.0 {
                info!("[PgSQL] Handled {} torrents", blacklist_handled_entries);
            }
        }
        info!("[PgSQL] Saved {} blacklisted torrents", blacklist_handled_entries);
        let _ = self.commit(blacklist_transaction).await;
        Ok(blacklist_handled_entries)
    }

    pub async fn load_keys(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error>
    {
        let mut start = 0u64;
        let length = 100000u64;
        let mut hashes = 0u64;
        let structure = match tracker.config.deref().clone().database_structure.clone().unwrap().keys {
            None => { return Err(Error::RowNotFound); }
            Some(db_structure) => { db_structure }
        };
        loop {
            info!(
                "[PgSQL] Trying to querying {} keys hashes - Skip: {}",
                length,
                start
            );
            let string_format = match tracker.config.deref().clone().database_structure.unwrap().keys.unwrap().bin_type_hash {
                true => {
                    format!(
                        "SELECT encode({}::bytea, 'hex'), {} FROM {} LIMIT {}, {}",
                        structure.column_hash,
                        structure.column_timeout,
                        structure.database_name,
                        start,
                        length
                    )
                }
                false => {
                    format!(
                        "SELECT {}, {} FROM {} LIMIT {}, {}",
                        structure.column_hash,
                        structure.column_timeout,
                        structure.database_name,
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
        }
        Ok(hashes)
    }

    pub async fn save_keys(&self, tracker: Arc<TorrentTracker>, keys: BTreeMap<InfoHash, i64>) -> Result<u64, Error>
    {
        let mut keys_transaction = self.pool.begin().await?;
        let mut keys_handled_entries = 0u64;
        let structure = match tracker.config.deref().clone().database_structure.clone().unwrap().keys {
            None => { return Err(Error::RowNotFound); }
            Some(db_structure) => { db_structure }
        };
        for (hash, timeout) in keys.iter() {
            keys_handled_entries += 1;
            let string_format = match tracker.config.deref().clone().database_structure.unwrap().keys.unwrap().bin_type_hash {
                true => {
                    format!(
                        "INSERT INTO {} ({}, {}) VALUES (decode('{}', 'hex'), {}) ON CONFLICT ({}) DO UPDATE SET {}=excluded.{}",
                        structure.database_name,
                        structure.column_hash,
                        structure.column_timeout,
                        hash,
                        timeout,
                        structure.column_hash,
                        structure.column_timeout,
                        structure.column_timeout
                    )
                }
                false => {
                    format!(
                        "INSERT INTO {} ({}, {}) VALUES ('{}', {}) ON CONFLICT ({}) DO UPDATE SET {}=excluded.{}",
                        structure.database_name,
                        structure.column_hash,
                        structure.column_timeout,
                        hash,
                        timeout,
                        structure.column_hash,
                        structure.column_timeout,
                        structure.column_timeout
                    )
                }
            };
            match sqlx::query(string_format.as_str()).execute(&mut *keys_transaction).await {
                Ok(_) => {}
                Err(e) => {
                    error!("[PgSQL] Error: {}", e.to_string());
                    return Err(e);
                }
            }
            if (keys_handled_entries as f64 / 1000f64).fract() == 0.0 {
                info!("[PgSQL] Handled {} keys", keys_handled_entries);
            }
        }
        info!("[PgSQL] Saved {} keys", keys_handled_entries);
        let _ = self.commit(keys_transaction).await;
        Ok(keys_handled_entries)
    }

    pub async fn load_users(&self, tracker: Arc<TorrentTracker>) -> Result<u64, Error>
    {
        let mut start = 0u64;
        let length = 100000u64;
        let mut hashes = 0u64;
        let structure = match tracker.config.deref().clone().database_structure.clone().unwrap().users {
            None => { return Err(Error::RowNotFound); }
            Some(db_structure) => { db_structure }
        };
        loop {
            info!(
                "[PgSQL] Trying to querying {} users - Skip: {}",
                length,
                start
            );
            let string_format = match tracker.config.deref().clone().database_structure.unwrap().users.unwrap().id_uuid {
                true => {
                    match tracker.config.deref().clone().database_structure.unwrap().users.unwrap().bin_type_key {
                        true => {
                            format!(
                                "SELECT {}, encode({}::bytea, 'hex'), {}, {}, {}, {}, {} FROM {} LIMIT {}, {}",
                                structure.column_uuid,
                                structure.column_key,
                                structure.column_uploaded,
                                structure.column_downloaded,
                                structure.column_completed,
                                structure.column_updated,
                                structure.column_active,
                                structure.database_name,
                                start,
                                length
                            )
                        }
                        false => {
                            format!(
                                "SELECT {}, {}, {}, {}, {}, {}, {} FROM {} LIMIT {}, {}",
                                structure.column_uuid,
                                structure.column_key,
                                structure.column_uploaded,
                                structure.column_downloaded,
                                structure.column_completed,
                                structure.column_updated,
                                structure.column_active,
                                structure.database_name,
                                start,
                                length
                            )
                        }
                    }
                }
                false => {
                    match tracker.config.deref().clone().database_structure.unwrap().users.unwrap().bin_type_key {
                        true => {
                            format!(
                                "SELECT {}, encode({}::bytea, 'hex'), {}, {}, {}, {}, {} FROM {} LIMIT {}, {}",
                                structure.column_id,
                                structure.column_key,
                                structure.column_uploaded,
                                structure.column_downloaded,
                                structure.column_completed,
                                structure.column_updated,
                                structure.column_active,
                                structure.database_name,
                                start,
                                length
                            )
                        }
                        false => {
                            format!(
                                "SELECT {}, {}, {}, {}, {}, {}, {} FROM {} LIMIT {}, {}",
                                structure.column_id,
                                structure.column_key,
                                structure.column_uploaded,
                                structure.column_downloaded,
                                structure.column_completed,
                                structure.column_updated,
                                structure.column_active,
                                structure.database_name,
                                start,
                                length
                            )
                        }
                    }
                }
            };
            let mut rows = sqlx::query(string_format.as_str()).fetch(&self.pool);
            while let Some(result) = rows.try_next().await? {
                let hash = match tracker.config.deref().clone().database_structure.unwrap().users.unwrap().id_uuid {
                    true => {
                        let uuid_data: &[u8] = result.get(structure.column_uuid.as_str());
                        let mut hasher = Sha1::new();
                        hasher.update(uuid_data);
                        let hashed = <[u8; 20]>::try_from(hasher.finalize().as_slice()).unwrap();
                        hashed
                    }
                    false => {
                        let id_data: &[u8] = result.get(structure.column_id.as_str());
                        let mut hasher = Sha1::new();
                        hasher.update(id_data);
                        let hashed = <[u8; 20]>::try_from(hasher.finalize().as_slice()).unwrap();
                        hashed
                    }
                };
                tracker.add_user(UserId(hash), UserEntryItem {
                    key: UserId::from_str(result.get(structure.column_key.as_str())).unwrap(),
                    user_id: match tracker.config.deref().clone().database_structure.unwrap().users.unwrap().id_uuid {
                        true => { None }
                        false => { Some(result.get::<i64, &str>(structure.column_id.as_str()) as u64) }
                    },
                    user_uuid: match tracker.config.deref().clone().database_structure.unwrap().users.unwrap().id_uuid {
                        true => { Some(result.get(structure.column_uuid.as_str())) }
                        false => { None }
                    },
                    uploaded: result.get::<i64, &str>(structure.column_uploaded.as_str()) as u64,
                    downloaded: result.get::<i64, &str>(structure.column_downloaded.as_str()) as u64,
                    completed: result.get::<i64, &str>(structure.column_completed.as_str()) as u64,
                    updated: result.get::<i32, &str>(structure.column_updated.as_str()) as u64,
                    active: result.get::<i16, &str>(structure.column_active.as_str()) as u8,
                    torrents_active: Default::default(),
                });
                hashes += 1;
            }
            start += length;
            if hashes < start {
                break;
            }
        }
        Ok(hashes)
    }

    pub async fn save_users(&self, tracker: Arc<TorrentTracker>, users: BTreeMap<UserId, UserEntryItem>) -> Result<(), Error>
    {
        let mut users_transaction = self.pool.begin().await?;
        let mut users_handled_entries = 0u64;
        let structure = match tracker.config.deref().clone().database_structure.clone().unwrap().users {
            None => { return Err(Error::RowNotFound); }
            Some(db_structure) => { db_structure }
        };
        for (_, user_entry_item) in users.iter() {
            users_handled_entries += 1;
            let string_format = match  tracker.config.deref().clone().database.unwrap().insert_vacant {
                true => {
                    match tracker.config.deref().clone().database_structure.unwrap().users.unwrap().id_uuid {
                        true => {
                            match tracker.config.deref().clone().database_structure.unwrap().users.unwrap().bin_type_key {
                                true => {
                                    format!(
                                        "INSERT INTO {} ({}, {}, {}, {}, {}, {}, {}) VALUES ('{}', {}, {}, {}, decode('{}', 'hex'), {}, {}) ON CONFLICT ({}) DO UPDATE SET {}=excluded.{}, {}=excluded.{}, {}=excluded.{}, {}=excluded.{}, {}=excluded.{}, {}=excluded.{}",
                                        structure.database_name,
                                        structure.column_uuid,
                                        structure.column_completed,
                                        structure.column_active,
                                        structure.column_downloaded,
                                        structure.column_key,
                                        structure.column_uploaded,
                                        structure.column_updated,
                                        user_entry_item.user_uuid.clone().unwrap(),
                                        user_entry_item.completed,
                                        user_entry_item.active,
                                        user_entry_item.downloaded,
                                        user_entry_item.key,
                                        user_entry_item.uploaded,
                                        user_entry_item.updated,
                                        structure.column_uuid,
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
                                false => {
                                    format!(
                                        "INSERT INTO {} ({}, {}, {}, {}, {}, {}, {}) VALUES ('{}', {}, {}, {}, '{}', {}, {}) ON CONFLICT ({}) DO UPDATE SET {}=excluded.{}, {}=excluded.{}, {}=excluded.{}, {}=excluded.{}, {}=excluded.{}, {}=excluded.{}",
                                        structure.database_name,
                                        structure.column_uuid,
                                        structure.column_completed,
                                        structure.column_active,
                                        structure.column_downloaded,
                                        structure.column_key,
                                        structure.column_uploaded,
                                        structure.column_updated,
                                        user_entry_item.user_uuid.clone().unwrap(),
                                        user_entry_item.completed,
                                        user_entry_item.active,
                                        user_entry_item.downloaded,
                                        user_entry_item.key,
                                        user_entry_item.uploaded,
                                        user_entry_item.updated,
                                        structure.column_uuid,
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
                        false => {
                            match tracker.config.deref().clone().database_structure.unwrap().users.unwrap().bin_type_key {
                                true => {
                                    format!(
                                        "INSERT INTO {} ({}, {}, {}, {}, {}, {}, {}) VALUES ('{}', {}, {}, {}, decode('{}', 'hex'), {}, {}) ON CONFLICT ({}) DO UPDATE SET {}=excluded.{}, {}=excluded.{}, {}=excluded.{}, {}=excluded.{}, {}=excluded.{}, {}=excluded.{}",
                                        structure.database_name,
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
                                        structure.column_id,
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
                                false => {
                                    format!(
                                        "INSERT INTO {} ({}, {}, {}, {}, {}, {}, {}) VALUES ('{}', {}, {}, {}, '{}', {}, {}) ON CONFLICT ({}) DO UPDATE SET {}=excluded.{}, {}=excluded.{}, {}=excluded.{}, {}=excluded.{}, {}=excluded.{}, {}=excluded.{}",
                                        structure.database_name,
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
                                        structure.column_id,
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
                    match tracker.config.deref().clone().database_structure.unwrap().users.unwrap().id_uuid {
                        true => {
                            match tracker.config.deref().clone().database_structure.unwrap().users.unwrap().bin_type_key {
                                true => {
                                    format!(
                                        "UPDATE {} SET {}={}, {}={}, {}={}, {}=decode('{}', 'hex'), {}={}, {}={} WHERE {}='{}' AND EXISTS (SELECT 1 FROM {} WHERE {}='{}')",
                                        structure.database_name,
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
                                        structure.database_name,
                                        structure.column_uuid,
                                        user_entry_item.user_uuid.clone().unwrap()
                                    )
                                }
                                false => {
                                    format!(
                                        "UPDATE {} SET {}={}, {}={}, {}={}, {}='{}', {}={}, {}={} WHERE {}='{}' AND EXISTS (SELECT 1 FROM {} WHERE {}='{}')",
                                        structure.database_name,
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
                                        structure.database_name,
                                        structure.column_uuid,
                                        user_entry_item.user_uuid.clone().unwrap()
                                    )
                                }
                            }
                        }
                        false => {
                            match tracker.config.deref().clone().database_structure.unwrap().users.unwrap().bin_type_key {
                                true => {
                                    format!(
                                        "UPDATE {} SET {}={}, {}={}, {}={}, {}=decode('{}', 'hex'), {}={}, {}={} WHERE {}='{}' AND EXISTS (SELECT 1 FROM {} WHERE {}='{}')",
                                        structure.database_name,
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
                                        structure.database_name,
                                        structure.column_id,
                                        user_entry_item.user_id.unwrap()
                                    )
                                }
                                false => {
                                    format!(
                                        "UPDATE {} SET {}={}, {}={}, {}={}, {}='{}', {}={}, {}={} WHERE {}='{}' AND EXISTS (SELECT 1 FROM {} WHERE {}='{}')",
                                        structure.database_name,
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
                                        structure.database_name,
                                        structure.column_id,
                                        user_entry_item.user_id.unwrap()
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
                    error!("[PgSQL] Error: {}", e.to_string());
                    return Err(e);
                }
            }
            if (users_handled_entries as f64 / 1000f64).fract() == 0.0 || users.len() as u64 == users_handled_entries {
                info!("[PgSQL] Handled {} users", users_handled_entries);
            }
        }
        self.commit(users_transaction).await
    }

    pub async fn reset_seeds_peers(&self, tracker: Arc<TorrentTracker>) -> Result<(), Error>
    {
        let mut reset_seeds_peers_transaction = self.pool.begin().await?;
        let structure = match tracker.config.deref().clone().database_structure.clone().unwrap().torrents {
            None => { return Err(Error::RowNotFound); }
            Some(db_structure) => { db_structure }
        };
        let string_format = format!(
            "UPDATE {} SET ({}, {}) = (0, 0)",
            structure.database_name,
            structure.column_seeds,
            structure.column_peers
        );
        match sqlx::query(string_format.as_str()).execute(&mut *reset_seeds_peers_transaction).await {
            Ok(_) => {}
            Err(e) => {
                error!("[PgSQL] Error: {}", e.to_string());
                return Err(e);
            }
        }
        let _ = self.commit(reset_seeds_peers_transaction).await;
        Ok(())
    }

    pub async fn commit(&self, transaction: Transaction<'_, Postgres>) -> Result<(), Error>
    {
        match transaction.commit().await {
            Ok(_) => {
                Ok(())
            }
            Err(e) => {
                error!("[PgSQL3] Error: {}", e.to_string());
                Err(e)
            }
        }
    }
}