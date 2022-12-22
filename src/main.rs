use std::env;
use std::net::SocketAddr;
use std::process::exit;
use std::time::Duration;
use axum_server::Handle;
use futures::future::try_join_all;
use log::{error, info};
use scc::ebr::Arc;
use torrust_axum::common::{tcp_check_host_and_port_used, udp_check_host_and_port_used};
use torrust_axum::config;
use torrust_axum::http_api::{http_api, https_api};
use torrust_axum::http_service::{http_service, https_service};
use torrust_axum::logging::setup_logging;
use torrust_axum::tracker::{StatsEvent, TorrentTracker};
use torrust_axum::udp_service::udp_service;

#[tokio::main]
async fn main() -> std::io::Result<()>
{
    let args: Vec<String> = env::args().collect();
    let mut config_create: bool = false;
    if args.iter().any(|i| i=="--create-config") {
        config_create = true;
    }

    let config = match config::Configuration::load_from_file(config_create) {
        Ok(config) => Arc::new(config),
        Err(_) => exit(101)
    };

    setup_logging(&config);

    info!("{} - Version: {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

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
    tokio::spawn(async move {
        let interval = Duration::from_secs(interval_peer_cleanup);
        let mut interval = tokio::time::interval(interval);
        interval.tick().await;
        loop {
            tracker_clone.clone().set_stats(StatsEvent::TimestampTimeout, chrono::Utc::now().timestamp() as i64 + tracker_clone.clone().config.peer_timeout.unwrap() as i64).await;
            interval.tick().await;
            info!("[PEERS] Checking now for dead peers.");
            tracker_clone.clone().clean_peers(Duration::from_secs(tracker_clone.clone().config.clone().peer_timeout.unwrap())).await;
            info!("[PEERS] Peers cleaned up.");
        }
    });

    if config.keys {
        let interval_keys_cleanup = config.clone().keys_cleanup_interval.unwrap_or(60);
        let tracker_clone = tracker.clone();
        tokio::spawn(async move {
            let interval = Duration::from_secs(interval_keys_cleanup);
            let mut interval = tokio::time::interval(interval);
            interval.tick().await;
            loop {
                tracker_clone.clone().set_stats(StatsEvent::TimestampKeysTimeout, chrono::Utc::now().timestamp() as i64 + tracker_clone.clone().config.keys_cleanup_interval.unwrap() as i64).await;
                interval.tick().await;
                info!("[KEYS] Checking now for old keys, and remove them.");
                tracker_clone.clone().clean_keys().await;
                info!("[KEYS] Keys cleaned up.");
            }
        });
    }

    let interval_persistence = config.clone().persistence_interval.unwrap_or(900);
    let tracker_clone = tracker.clone();
    tokio::spawn(async move {
        let interval = Duration::from_secs(interval_persistence);
        let mut interval = tokio::time::interval(interval);
        interval.tick().await;
        loop {
            tracker_clone.clone().set_stats(StatsEvent::TimestampSave, chrono::Utc::now().timestamp() as i64 + tracker_clone.clone().config.persistence_interval.unwrap() as i64).await;
            interval.tick().await;
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

    if config.statistics_enabled {
        let console_log_interval = config.clone().log_console_interval.unwrap();
        let tracker_clone = tracker.clone();
        tokio::spawn(async move {
            let interval = Duration::from_secs(console_log_interval);
            let mut interval = tokio::time::interval(interval);
            loop {
                tracker_clone.clone().set_stats(StatsEvent::TimestampConsole, chrono::Utc::now().timestamp() as i64 + tracker_clone.clone().config.log_console_interval.unwrap() as i64).await;
                interval.tick().await;
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
