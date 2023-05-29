use async_std::task;
use clap::Parser;
use futures::future::try_join_all;
use log::{error, info};
use scc::ebr::Arc;
use std::alloc::System;
use std::env;
use std::net::SocketAddr;
use std::process::exit;
use std::time::Duration;
use tokio::time::timeout;

use torrust_axum::common::{tcp_check_host_and_port_used, udp_check_host_and_port_used};
use torrust_axum::config::{Configuration, DatabaseStructureConfig};
use torrust_axum::databases::DatabaseDrivers;
use torrust_axum::http_api::{http_api, https_api};
use torrust_axum::http_service::{http_service, https_service};
use torrust_axum::logging::setup_logging;
use torrust_axum::tracker::TorrentTracker;
use torrust_axum::tracker_objects::stats::StatsEvent;
use torrust_axum::udp_service::udp_service;

#[global_allocator]
static A: System = System;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Create config.toml file if not exists or is broken.
    #[arg(long)]
    create_config: bool,

    /// Convert SQLite3, MySQL or PgSQL database to any of the other.
    #[arg(long)]
    convert_database: bool,

    /// Which source engine to use.
    #[arg(long, value_enum, required = false)]
    source_engine: Option<DatabaseDrivers>,

    /// Source database: 'sqlite://...', 'mysql://...', 'pgsql://...'.
    #[arg(long, required = false)]
    source: Option<String>,

    /// Which destination engine to use.
    #[arg(long, value_enum, required = false)]
    destination_engine: Option<DatabaseDrivers>,

    /// Destination database: 'sqlite://...', 'mysql://...', 'pgsql://...'.
    #[arg(long, required = false)]
    destination: Option<String>,
}

#[tokio::main]
async fn main() -> std::io::Result<()>
{
    let args = Cli::parse();

    let mut create_config: bool = false;
    if args.create_config {
        create_config = true;
    }

    let config = match Configuration::load_from_file(create_config) {
        Ok(config) => Arc::new(config),
        Err(_) => exit(101)
    };

    setup_logging(&config.clone());

    info!("{} - Version: {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    if args.convert_database {
        info!("Database Conversion execute.");

        if args.source_engine.clone().is_none() || args.source.clone().is_none() || args.destination_engine.clone().is_none() || args.destination.clone().is_none() {
            error!("Need Source/Destination Engine and URI to execute!");
            exit(1);
        }

        info!("[RETRIEVE] [Engine: {:?}] [URI: {}]", args.source_engine.clone().unwrap(), args.source.clone().unwrap());

        let source_engine = args.source_engine.clone().unwrap();
        let source = args.source.clone().unwrap();
        let destination_engine = args.destination_engine.clone().unwrap();
        let destination = args.destination.clone().unwrap();
        let config_retrieve_db_structure = config.db_structure.clone();
        let config_send_db_structure = config.db_structure.clone();

        let tracker_receive = Arc::new(TorrentTracker::new(Arc::new(Configuration {
            log_level: "".to_string(),
            log_console_interval: None,
            statistics_enabled: false,
            global_check_interval: None,
            db_driver: source_engine,
            db_path: source,
            persistence: true,
            persistence_interval: None,
            api_key: "".to_string(),
            whitelist: config.whitelist,
            blacklist: config.blacklist,
            keys: config.keys,
            keys_cleanup_interval: None,
            maintenance_mode_enabled: false,
            interval: None,
            interval_minimum: None,
            peer_timeout: None,
            peers_returned: None,
            interval_cleanup: None,
            cleanup_chunks: None,
            udp_server: vec![],
            http_server: vec![],
            api_server: vec![],
            db_structure: DatabaseStructureConfig {
                db_torrents: config_retrieve_db_structure.db_torrents,
                table_torrents_info_hash: config_retrieve_db_structure.table_torrents_info_hash,
                table_torrents_completed: config_retrieve_db_structure.table_torrents_completed,
                db_whitelist: config_retrieve_db_structure.db_whitelist,
                table_whitelist_info_hash: config_retrieve_db_structure.table_whitelist_info_hash,
                db_blacklist: config_retrieve_db_structure.db_blacklist,
                table_blacklist_info_hash: config_retrieve_db_structure.table_blacklist_info_hash,
                db_keys: config_retrieve_db_structure.db_keys,
                table_keys_hash: config_retrieve_db_structure.table_keys_hash,
                table_keys_timeout: config_retrieve_db_structure.table_keys_timeout,
            },
        }).clone()).await);
        tracker_receive.clone().load_torrents(tracker_receive.clone()).await;

        let tracker_send = Arc::new(TorrentTracker::new(Arc::new(Configuration {
            log_level: "".to_string(),
            log_console_interval: None,
            statistics_enabled: false,
            global_check_interval: None,
            db_driver: destination_engine,
            db_path: destination,
            persistence: true,
            persistence_interval: None,
            api_key: "".to_string(),
            whitelist: config.whitelist,
            blacklist: config.blacklist,
            keys: config.keys,
            keys_cleanup_interval: None,
            maintenance_mode_enabled: false,
            interval: None,
            interval_minimum: None,
            peer_timeout: None,
            peers_returned: None,
            interval_cleanup: None,
            cleanup_chunks: None,
            udp_server: vec![],
            http_server: vec![],
            api_server: vec![],
            db_structure: DatabaseStructureConfig {
                db_torrents: config_send_db_structure.db_torrents,
                table_torrents_info_hash: config_send_db_structure.table_torrents_info_hash,
                table_torrents_completed: config_send_db_structure.table_torrents_completed,
                db_whitelist: config_send_db_structure.db_whitelist,
                table_whitelist_info_hash: config_send_db_structure.table_whitelist_info_hash,
                db_blacklist: config_send_db_structure.db_blacklist,
                table_blacklist_info_hash: config_send_db_structure.table_blacklist_info_hash,
                db_keys: config_send_db_structure.db_keys,
                table_keys_hash: config_send_db_structure.table_keys_hash,
                table_keys_timeout: config_send_db_structure.table_keys_timeout,
            },
        }).clone()).await);

        info!("[SEND] [Engine: {:?}] [URI: {}]", args.destination_engine.clone().unwrap(), args.destination.clone().unwrap());
        let mut start: u64 = 0;
        let amount: u64 = 100000;
        loop {
            let torrents_block = match tracker_receive.get_torrents_chunk(start, amount).await {
                Ok(data_request) => { data_request }
                Err(_) => { continue; }
            };
            if torrents_block.is_empty() {
                break;
            }
            for (info_hash, completed) in torrents_block.iter() {
                tracker_send.add_shadow(*info_hash, *completed).await;
                tracker_receive.remove_torrent(*info_hash, false).await;
            }
            start += amount;
        }

        let _ = tracker_send.save_torrents().await;

        exit(0);
    }

    let tracker = Arc::new(TorrentTracker::new(config.clone()).await);

    // Load torrents
    if config.persistence {
        tracker.clone().load_torrents(tracker.clone()).await;
        if config.whitelist {
            tracker.clone().load_whitelists().await;
        }
        if config.blacklist {
            tracker.clone().load_blacklists().await;
        }
        if config.keys {
            tracker.clone().load_keys().await;
        }
    }

    let mut api_handlers = Vec::new();
    let mut api_futures = Vec::new();
    let mut apis_handlers = Vec::new();
    let mut apis_futures = Vec::new();

    // let mut apis_futures = Vec::new();
    for api_server_object in &config.api_server {
        if api_server_object.enabled {
            tcp_check_host_and_port_used(api_server_object.bind_address.clone());
            let address: SocketAddr = api_server_object.bind_address.parse().unwrap();
            let tracker_clone = tracker.clone();
            if api_server_object.ssl {
                let (handle, https_api) = https_api(address, tracker_clone, api_server_object.ssl_key.clone(), api_server_object.ssl_cert.clone()).await;
                apis_handlers.push(handle);
                apis_futures.push(https_api);
            } else {
                let (handle, http_api) = http_api(address, tracker_clone).await;
                api_handlers.push(handle);
                api_futures.push(http_api);
            }
        }
    }

    let mut http_handlers = Vec::new();
    let mut http_futures = Vec::new();
    let mut https_handlers = Vec::new();
    let mut https_futures = Vec::new();

    for http_server_object in &config.http_server {
        if http_server_object.enabled {
            tcp_check_host_and_port_used(http_server_object.bind_address.clone());
            let address: SocketAddr = http_server_object.bind_address.parse().unwrap();
            let tracker_clone = tracker.clone();
            if http_server_object.ssl {
                let (handle, https_service) = https_service(address, tracker_clone, http_server_object.ssl_key.clone(), http_server_object.ssl_cert.clone()).await;
                https_handlers.push(handle);
                https_futures.push(https_service);
            } else {
                let (handle, http_service) = http_service(address, tracker_clone).await;
                http_handlers.push(handle);
                http_futures.push(http_service);
            }
        }
    }

    let (udp_tx, udp_rx) = tokio::sync::watch::channel(false);
    let mut udp_futures = Vec::new();
    for udp_server_object in &config.udp_server {
        if udp_server_object.enabled {
            udp_check_host_and_port_used(udp_server_object.bind_address.clone());
            let address: SocketAddr = udp_server_object.bind_address.parse().unwrap();
            let tracker_clone = tracker.clone();
            udp_futures.push(udp_service(address, tracker_clone, udp_rx.clone()).await);
        }
    }

    if !api_futures.is_empty() {
        tokio::spawn(async move {
            let _ = try_join_all(api_futures).await;
        });
    }

    if !apis_futures.is_empty() {
        tokio::spawn(async move {
            let _ = try_join_all(apis_futures).await;
        });
    }

    if !http_futures.is_empty() {
        tokio::spawn(async move {
            let _ = try_join_all(http_futures).await;
        });
    }

    if !https_futures.is_empty() {
        tokio::spawn(async move {
            let _ = try_join_all(https_futures).await;
        });
    }

    // Schedule system, instead of each system in their own thread.
    // This must prevent the system locking itself up when doing too much or already active.

    let tracker_clone = tracker.clone();
    tokio::spawn(async move {
        // Set the timestamps for each action in the stats, this will be used to execute the activity.
        tracker_clone.set_stats(StatsEvent::TimestampKeysTimeout, chrono::Utc::now().timestamp() + tracker_clone.config.keys_cleanup_interval.unwrap() as i64).await;
        tracker_clone.set_stats(StatsEvent::TimestampTimeout, chrono::Utc::now().timestamp() + tracker_clone.config.interval_cleanup.unwrap() as i64).await;
        tracker_clone.set_stats(StatsEvent::TimestampSave, chrono::Utc::now().timestamp() + tracker_clone.config.persistence_interval.unwrap() as i64).await;

        // Here we run the scheduler action.
        loop {
            task::sleep(Duration::from_secs(tracker_clone.config.global_check_interval.unwrap_or(10))).await;

            // Check if we need to run the keys cleanup.
            let tracker_clone_clone = tracker_clone.clone();
            if tracker_clone.config.keys && chrono::Utc::now().timestamp() > tracker_clone.get_stats().await.timestamp_run_keys_timeout {
                info!("[KEYS] Checking now for old keys, and remove them.");
                tracker_clone_clone.clean_keys().await;
                tracker_clone_clone.set_stats(StatsEvent::TimestampKeysTimeout, chrono::Utc::now().timestamp() + tracker_clone_clone.config.keys_cleanup_interval.unwrap() as i64).await;
                info!("[KEYS] Keys cleaned up.");
            }

            // Check if we need to run the Peers cleanup.
            let tracker_clone_clone = tracker_clone.clone();
            if chrono::Utc::now().timestamp() > tracker_clone.get_stats().await.timestamp_run_timeout {
                tokio::spawn(timeout(Duration::from_secs(tracker_clone.config.interval_cleanup.unwrap_or(30)), async move {
                    info!("[PEERS] Checking now for dead peers.");
                    tracker_clone_clone.clean_peers(Duration::from_secs(tracker_clone_clone.config.clone().peer_timeout.unwrap())).await;
                    tracker_clone_clone.set_stats(StatsEvent::TimestampTimeout, chrono::Utc::now().timestamp() + tracker_clone_clone.config.interval_cleanup.unwrap() as i64).await;
                    info!("[PEERS] Peers cleaned up.");
                }));
            }

            // Check if we need to run the Save Data code.
            let tracker_clone_clone = tracker_clone.clone();
            if tracker_clone.config.persistence && chrono::Utc::now().timestamp() > tracker_clone.get_stats().await.timestamp_run_save {
                tokio::spawn(timeout(Duration::from_secs(tracker_clone.config.persistence_interval.unwrap_or(30)), async move {
                    info!("[SAVING] Starting persistence saving procedure.");
                    info!("[SAVING] Moving Updates to Shadow...");
                    tracker_clone_clone.transfer_updates_to_shadow().await;
                    info!("[SAVING] Saving data from Shadow to database...");
                    if let Ok(save_stat) = tracker_clone_clone.save_torrents().await {
                        if save_stat {
                            info!("[SAVING] Clearing shadow, saving procedure finishing...");
                            tracker_clone_clone.clear_shadow().await;
                            info!("[SAVING] Torrents saved.");
                        } else {
                            error!("[SAVING] An error occurred while saving data...");
                        }
                    } else {
                        error!("[SAVING] An error occurred while saving data, lock issue...");
                    }
                    if tracker_clone_clone.config.whitelist {
                        info!("[SAVING] Saving data from Whitelist to database...");
                        if tracker_clone_clone.save_whitelists().await {
                            info!("[SAVING] Whitelists saved.");
                        } else {
                            error!("[SAVING] An error occurred while saving data...");
                        }
                    }
                    if tracker_clone_clone.config.blacklist {
                        info!("[SAVING] Saving data from Blacklist to database...");
                        if tracker_clone_clone.save_blacklists().await {
                            info!("[SAVING] Blacklists saved.");
                        } else {
                            error!("[SAVING] An error occurred while saving data...");
                        }
                    }
                    if tracker_clone_clone.config.keys {
                        info!("[SAVING] Saving data from Keys to database...");
                        if tracker_clone_clone.save_keys().await {
                            info!("[SAVING] Keys saved.");
                        } else {
                            error!("[SAVING] An error occurred while saving data...");
                        }
                    }
                    tracker_clone_clone.set_stats(StatsEvent::TimestampSave, chrono::Utc::now().timestamp() + tracker_clone_clone.config.persistence_interval.unwrap() as i64).await;
                    info!("[SAVING] Saving persistent data procedure done.");
                }));
            }
        }
    });

    if config.statistics_enabled {
        let tracker_clone = tracker.clone();
        tokio::spawn(async move {
            loop {
                tracker_clone.set_stats(StatsEvent::TimestampConsole, chrono::Utc::now().timestamp() + tracker_clone.config.log_console_interval.unwrap() as i64).await;
                task::sleep(Duration::from_secs(tracker_clone.config.log_console_interval.unwrap_or(30))).await;
                let stats = tracker_clone.clone().get_stats().await;
                info!("[STATS] Torrents: {} - Updates: {} - Shadow {}: - Seeds: {} - Peers: {} - Completed: {}", stats.torrents, stats.torrents_updates, stats.torrents_shadow, stats.seeds, stats.peers, stats.completed);
                info!("[STATS] Whitelists: {} - Blacklists: {} - Keys: {}", stats.whitelist, stats.blacklist, stats.keys);
                info!("[STATS TCP IPv4] Connect: {} - API: {} - Announce: {} - Scrape: {}", stats.tcp4_connections_handled, stats.tcp4_api_handled, stats.tcp4_announces_handled, stats.tcp4_scrapes_handled);
                info!("[STATS TCP IPv6] Connect: {} - API: {} - Announce: {} - Scrape: {}", stats.tcp6_connections_handled, stats.tcp6_api_handled, stats.tcp6_announces_handled, stats.tcp6_scrapes_handled);
                info!("[STATS UDP IPv4] Connect: {} - Announce: {} - Scrape: {}", stats.udp4_connections_handled, stats.udp4_announces_handled, stats.udp4_scrapes_handled);
                info!("[STATS UDP IPv6] Connect: {} - Announce: {} - Scrape: {}", stats.udp6_connections_handled, stats.udp6_announces_handled, stats.udp6_scrapes_handled);
            }
        });
    }

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Shutdown request received, shutting down...");
            let _ = udp_tx.send(true);
            let _ = futures::future::join_all(udp_futures).await;
            for handle in api_handlers.iter() {
                handle.stop(true).await;
            }
            for handle in apis_handlers.iter() {
                handle.stop(true).await;
            }
            for handle in http_handlers.iter() {
                handle.stop(true).await;
            }
            for handle in https_handlers.iter() {
                handle.stop(true).await;
            }
            if tracker.clone().config.persistence {
                info!("[SAVING] Starting persistence saving procedure.");
                info!("[SAVING] Moving Updates to Shadow...");
                tracker.clone().transfer_updates_to_shadow().await;
                info!("[SAVING] Saving data from Torrents to database...");
                if let Ok(save_stat) = tracker.clone().save_torrents().await {
                    if save_stat {
                        info!("[SAVING] Clearing shadow, saving procedure finishing...");
                        tracker.clone().clear_shadow().await;
                        info!("[SAVING] Torrents saved.");
                    } else {
                        error!("[SAVING] An error occurred while saving data...");
                    }
                } else {
                    error!("[SAVING] An error occurred while saving data, lock issue...");
                }
                if config.whitelist {
                    info!("[SAVING] Saving data from Whitelist to database...");
                    if tracker.clone().save_whitelists().await {
                        info!("[SAVING] Whitelists saved.");
                    } else {
                        error!("[SAVING] An error occurred while saving data...");
                    }
                }
                if config.blacklist {
                    info!("[SAVING] Saving data from Blacklist to database...");
                    if tracker.clone().save_blacklists().await {
                        info!("[SAVING] Blacklists saved.");
                    } else {
                        error!("[SAVING] An error occurred while saving data...");
                    }
                }
                if config.keys {
                    info!("[SAVING] Saving data from Keys to database...");
                    if tracker.clone().save_keys().await {
                        info!("[SAVING] Keys saved.");
                    } else {
                        error!("[SAVING] An error occurred while saving data...");
                    }
                }
            }
            info!("Server shutting down completed");
            Ok(())
        }
    }
}
