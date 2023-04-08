use std::env;
use std::net::SocketAddr;
use std::process::exit;
use std::sync::mpsc;
use std::time::Duration;
use axum_server::Handle;
use clap::Parser;
use futures::future::try_join_all;
use log::{error, info};
use scc::ebr::Arc;
use torrust_axum::common::{tcp_check_host_and_port_used, udp_check_host_and_port_used};
use torrust_axum::config;
use torrust_axum::config::{Configuration, DatabaseStructureConfig};
use torrust_axum::databases::DatabaseDrivers;
use torrust_axum::http_api::{http_api, https_api};
use torrust_axum::http_service::{http_service, https_service};
use torrust_axum::logging::setup_logging;
use torrust_axum::tracker::{StatsEvent, TorrentTracker};
use torrust_axum::udp_service::udp_service;

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

    let config = match config::Configuration::load_from_file(create_config) {
        Ok(config) => Arc::new(config),
        Err(_) => exit(101)
    };

    setup_logging(&config.clone());

    info!("{} - Version: {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    if args.convert_database.clone() {
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
            interval_cleanup: None,
            peer_timeout: None,
            peers_returned: None,
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
        tracker_receive.clone().load_torrents().await;

        let tracker_send = Arc::new(TorrentTracker::new(Arc::new(Configuration {
            log_level: "".to_string(),
            log_console_interval: None,
            statistics_enabled: false,
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
            interval_cleanup: None,
            peer_timeout: None,
            peers_returned: None,
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
        let start: u64 = 0;
        let amount: u64 = 100000;
        loop {
            let torrents_block = tracker_receive.get_torrents(start, amount).await;
            if torrents_block.is_empty() {
                break;
            }
            tracker_send.add_shadow(torrents_block.clone()).await;
            for (info_hash, _completed) in torrents_block.iter() {
                tracker_receive.remove_torrent(*info_hash, false).await;
            }
        }

        tracker_send.save_torrents().await;


        // tracker.clone().copy_torrents_to_shadow().await;
        // tracker.clone().save_torrents().await;

        exit(0);
    }

    let tracker = Arc::new(TorrentTracker::new(config.clone()).await);


    // Load torrents
    if config.persistence {
        tracker.clone().load_torrents().await;
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

    let handle = Handle::new();

    let mut api_futures = Vec::new();
    let mut apis_futures = Vec::new();
    for api_server_object in &config.api_server {
        if api_server_object.enabled {
            tcp_check_host_and_port_used(api_server_object.bind_address.clone());
            let address: SocketAddr = api_server_object.bind_address.parse().unwrap();
            let handle = handle.clone();
            let tracker_clone = tracker.clone();
            if api_server_object.ssl {
                apis_futures.push(https_api(handle.clone(), address, tracker_clone, api_server_object.ssl_key.clone(), api_server_object.ssl_cert.clone()).await);
            } else {
                api_futures.push(http_api(handle.clone(), address, tracker_clone).await);
            }
        }
    }

    let mut http_futures = Vec::new();
    let mut https_futures = Vec::new();
    for http_server_object in &config.http_server {
        if http_server_object.enabled {
            tcp_check_host_and_port_used(http_server_object.bind_address.clone());
            let address: SocketAddr = http_server_object.bind_address.parse().unwrap();
            let handle = handle.clone();
            let tracker_clone = tracker.clone();
            if http_server_object.ssl {
                https_futures.push(https_service(handle.clone(), address, tracker_clone, http_server_object.ssl_key.clone(), http_server_object.ssl_cert.clone()).await);
            } else {
                http_futures.push(http_service(handle.clone(), address, tracker_clone).await);
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

    let interval_peer_cleanup = config.clone().interval_cleanup.unwrap_or(900);
    let tracker_clone = tracker.clone();
    let (peer_cleanup_send, peer_cleanup_recv) = mpsc::channel();
    tokio::spawn(async move {
        loop {
            tracker_clone.clone().set_stats(StatsEvent::TimestampTimeout, chrono::Utc::now().timestamp() as i64 + tracker_clone.clone().config.peer_timeout.unwrap() as i64).await;
            if let Ok(_) = peer_cleanup_recv.recv_timeout(Duration::from_secs(interval_peer_cleanup)) { break; }
            info!("[PEERS] Checking now for dead peers.");
            tracker_clone.clone().clean_peers(Duration::from_secs(tracker_clone.clone().config.clone().peer_timeout.unwrap())).await;
            info!("[PEERS] Peers cleaned up.");
        }
    });

    let (keys_cleanup_send, keys_cleanup_recv) = mpsc::channel();
    if config.keys {
        let interval_keys_cleanup = config.clone().keys_cleanup_interval.unwrap_or(60);
        let tracker_clone = tracker.clone();
        tokio::spawn(async move {
            loop {
                tracker_clone.clone().set_stats(StatsEvent::TimestampKeysTimeout, chrono::Utc::now().timestamp() as i64 + tracker_clone.clone().config.keys_cleanup_interval.unwrap() as i64).await;
                if let Ok(_) = keys_cleanup_recv.recv_timeout(Duration::from_secs(interval_keys_cleanup)) { break; }
                info!("[KEYS] Checking now for old keys, and remove them.");
                tracker_clone.clone().clean_keys().await;
                info!("[KEYS] Keys cleaned up.");
            }
        });
    }

    let interval_persistence = config.clone().persistence_interval.unwrap_or(900);
    let tracker_clone = tracker.clone();
    let (persistence_send, persistence_recv) = mpsc::channel();
    tokio::spawn(async move {
        loop {
            tracker_clone.clone().set_stats(StatsEvent::TimestampSave, chrono::Utc::now().timestamp() as i64 + tracker_clone.clone().config.persistence_interval.unwrap() as i64).await;
            if let Ok(_) = persistence_recv.recv_timeout(Duration::from_secs(interval_persistence)) { break; }
            info!("[SAVING] Starting persistence saving procedure.");
            info!("[SAVING] Moving Updates to Shadow...");
            tracker_clone.clone().transfer_updates_to_shadow().await;
            info!("[SAVING] Saving data from Shadow to database...");
            if tracker_clone.clone().save_torrents().await {
                info!("[SAVING] Clearing shadow, saving procedure finishing...");
                tracker_clone.clone().clear_shadow().await;
                info!("[SAVING] Torrents saved.");
            } else {
                error!("[SAVING] An error occurred while saving data...");
            }
            if tracker_clone.clone().config.whitelist {
                info!("[SAVING] Saving data from Whitelist to database...");
                if tracker_clone.clone().save_whitelists().await {
                    info!("[SAVING] Whitelists saved.");
                } else {
                    error!("[SAVING] An error occurred while saving data...");
                }
            }
            if tracker_clone.clone().config.blacklist {
                info!("[SAVING] Saving data from Blacklist to database...");
                if tracker_clone.clone().save_blacklists().await {
                    info!("[SAVING] Blacklists saved.");
                } else {
                    error!("[SAVING] An error occurred while saving data...");
                }
            }
            if tracker_clone.clone().config.keys {
                info!("[SAVING] Saving data from Keys to database...");
                if tracker_clone.clone().save_keys().await {
                    info!("[SAVING] Keys saved.");
                } else {
                    error!("[SAVING] An error occurred while saving data...");
                }
            }
        }
    });

    let (console_log_send, console_log_recv) = mpsc::channel();
    if config.statistics_enabled {
        let console_log_interval = config.clone().log_console_interval.unwrap();
        let tracker_clone = tracker.clone();
        tokio::spawn(async move {
            loop {
                tracker_clone.clone().set_stats(StatsEvent::TimestampConsole, chrono::Utc::now().timestamp() as i64 + tracker_clone.clone().config.log_console_interval.unwrap() as i64).await;
                if let Ok(_) = console_log_recv.recv_timeout(Duration::from_secs(console_log_interval)) { break; }
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
            handle.shutdown();
            let _ = udp_tx.send(true);
            let _ = futures::future::join_all(udp_futures);
            let _ = peer_cleanup_send.send(());
            let _ = keys_cleanup_send.send(());
            let _ = persistence_send.send(());
            let _ = console_log_send.send(());
            if tracker.clone().config.persistence {
                info!("[SAVING] Starting persistence saving procedure.");
                info!("[SAVING] Moving Updates to Shadow...");
                tracker.clone().transfer_updates_to_shadow().await;
                    info!("[SAVING] Saving data from Torrents to database...");
                if tracker.clone().save_torrents().await {
                    info!("[SAVING] Clearing shadow, saving procedure finishing...");
                    tracker.clone().clear_shadow().await;
                    info!("[SAVING] Torrents saved.");
                } else {
                    error!("[SAVING] An error occurred while saving data...");
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
