use std::net::SocketAddr;
use std::process::exit;
use std::sync::Arc;
use std::time::Duration;
use async_std::task;
use clap::Parser;
use futures_util::future::{try_join_all, TryJoinAll};
use log::info;
use parking_lot::deadlock;
use torrust_actix::api::api::api_service;
use torrust_actix::common::common::{setup_logging, udp_check_host_and_port_used};
use torrust_actix::config::structs::configuration::Configuration;
use torrust_actix::http::http::{http_check_host_and_port_used, http_service};
use torrust_actix::stats::enums::stats_event::StatsEvent;
use torrust_actix::tracker::structs::torrent_tracker::TorrentTracker;
// use torrust_actix::udp::udp::udp_service;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Create config.toml file if not exists or is broken.
    #[arg(long)]
    create_config: bool
}

#[tokio::main]
async fn main() -> std::io::Result<()>
{
    let args = Cli::parse();

    let config = match Configuration::load_from_file(args.create_config) {
        Ok(config) => Arc::new(config),
        Err(_) => exit(101)
    };

    setup_logging(&config);

    info!("{} - Version: {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    let tracker = Arc::new(TorrentTracker::new(config.clone()).await);

    tracker.set_stats(StatsEvent::Completed, config.total_downloads as i64);

    tokio::spawn(async move {
        loop {
            task::sleep(Duration::from_secs(10)).await;
            let deadlocks = deadlock::check_deadlock();
            if deadlocks.is_empty() {
                continue;
            }
            info!("[DEADLOCK] Found {} deadlocks", deadlocks.len());
            for (i, threads) in deadlocks.iter().enumerate() {
                info!("[DEADLOCK] #{}", i);
                for t in threads {
                    info!("[DEADLOCK] Thread ID: {:#?}", t.thread_id());
                    info!("[DEADLOCK] {:#?}", t.backtrace());
                }
            }
        }
    });

    let mut api_handlers = Vec::new();
    let mut api_futures = Vec::new();
    let mut apis_handlers = Vec::new();
    let mut apis_futures = Vec::new();
    for api_server_object in &config.api_server {
        if api_server_object.enabled {
            http_check_host_and_port_used(api_server_object.bind_address.clone());
            let address: SocketAddr = api_server_object.bind_address.parse().unwrap();
            if api_server_object.ssl {
                let (handle, https_api) = api_service(
                    address,
                    tracker.clone(),
                    tracker.config.api_keep_alive,
                    tracker.config.api_request_timeout,
                    tracker.config.api_disconnect_timeout,
                    api_server_object.threads.unwrap_or(std::thread::available_parallelism().unwrap().get() as u64),
                    (
                        api_server_object.ssl,
                        Some(api_server_object.ssl_key.clone()),
                        Some(api_server_object.ssl_cert.clone())
                    )
                ).await;
                apis_handlers.push(handle);
                apis_futures.push(https_api);
            } else {
                let (handle, http_api) = api_service(
                    address,
                    tracker.clone(),
                    tracker.config.api_keep_alive,
                    tracker.config.api_request_timeout,
                    tracker.config.api_disconnect_timeout,
                    api_server_object.threads.unwrap_or(std::thread::available_parallelism().unwrap().get() as u64),
                    (
                        api_server_object.ssl,
                        None,
                        None
                    )
                ).await;
                api_handlers.push(handle);
                api_futures.push(http_api);
            }
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

    let mut http_handlers = Vec::new();
    let mut http_futures = Vec::new();
    let mut https_handlers = Vec::new();
    let mut https_futures = Vec::new();
    for http_server_object in &config.http_server {
        if http_server_object.enabled {
            http_check_host_and_port_used(http_server_object.bind_address.clone());
            let address: SocketAddr = http_server_object.bind_address.parse().unwrap();
            if http_server_object.ssl {
                let (handle, https_service) = http_service(
                    address,
                    tracker.clone(),
                    tracker.config.http_keep_alive,
                    tracker.config.http_request_timeout,
                    tracker.config.http_disconnect_timeout,
                    http_server_object.threads.unwrap_or(std::thread::available_parallelism().unwrap().get() as u64),
                    (
                        http_server_object.ssl,
                        Some(http_server_object.ssl_key.clone()),
                        Some(http_server_object.ssl_cert.clone())
                    )
                ).await;
                https_handlers.push(handle);
                https_futures.push(https_service);
            } else {
                let (handle, http_service) = http_service(
                    address,
                    tracker.clone(),
                    tracker.config.http_keep_alive,
                    tracker.config.http_request_timeout,
                    tracker.config.http_disconnect_timeout,
                    http_server_object.threads.unwrap_or(std::thread::available_parallelism().unwrap().get() as u64),
                    (
                        http_server_object.ssl,
                        None,
                        None
                    )
                ).await;
                http_handlers.push(handle);
                http_futures.push(http_service);
            }
        }
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

    // let (udp_tx, udp_rx) = tokio::sync::watch::channel(false);
    // let mut udp_futures = Vec::new();
    // for udp_server_object in &config.udp_server {
    //     if udp_server_object.enabled {
    //         udp_check_host_and_port_used(udp_server_object.bind_address.clone());
    //         let address: SocketAddr = udp_server_object.bind_address.parse().unwrap();
    //         let tracker_clone = tracker.clone();
    //         udp_futures.push(udp_service(address, tracker_clone, udp_rx.clone()).await);
    //     }
    // }

    let tracker_spawn_stats = tracker.clone();
    tokio::spawn(async move {
        loop {
            tracker_spawn_stats.set_stats(StatsEvent::TimestampSave, chrono::Utc::now().timestamp() + 60i64);
            task::sleep(Duration::from_secs(tracker_spawn_stats.config.log_console_interval.unwrap_or(60u64))).await;
            let stats = tracker_spawn_stats.get_stats();
            info!("[STATS] Torrents: {} - Updates: {} - Shadow {}: - Seeds: {} - Peers: {} - Completed: {}", stats.torrents, stats.torrents_updates, stats.torrents_shadow, stats.seeds, stats.peers, stats.completed);
            info!("[STATS] Whitelists: {} - Blacklists: {} - Keys: {}", stats.whitelist, stats.blacklist, stats.keys);
            info!("[STATS TCP IPv4] Connect: {} - API: {} - A: {} - S: {} - F: {} - 404: {}", stats.tcp4_connections_handled, stats.tcp4_api_handled, stats.tcp4_announces_handled, stats.tcp4_scrapes_handled, stats.tcp4_failure, stats.tcp4_not_found);
            info!("[STATS TCP IPv6] Connect: {} - API: {} - A: {} - S: {} - F: {} - 404: {}", stats.tcp6_connections_handled, stats.tcp6_api_handled, stats.tcp6_announces_handled, stats.tcp6_scrapes_handled, stats.tcp6_failure, stats.tcp6_not_found);
            info!("[STATS UDP IPv4] Connect: {} - Announce: {} - Scrape: {}", stats.udp4_connections_handled, stats.udp4_announces_handled, stats.udp4_scrapes_handled);
            info!("[STATS UDP IPv6] Connect: {} - Announce: {} - Scrape: {}", stats.udp6_connections_handled, stats.udp6_announces_handled, stats.udp6_scrapes_handled);
        }
    });

    let tracker_spawn_cleanup = tracker.clone();
    tokio::spawn(async move {
        loop {
            tracker_spawn_cleanup.set_stats(StatsEvent::TimestampTimeout, chrono::Utc::now().timestamp() + tracker_spawn_cleanup.config.interval_cleanup.unwrap() as i64);
            task::sleep(Duration::from_secs(tracker_spawn_cleanup.config.interval_cleanup.unwrap_or(60))).await;
            info!("[PEERS] Checking now for dead peers.");
            let _ = tracker_spawn_cleanup.torrent_peers_cleanup(Duration::from_secs(tracker_spawn_cleanup.config.clone().peer_timeout.unwrap()), tracker_spawn_cleanup.config.persistence);
            info!("[PEERS] Peers cleaned up.");

            if tracker_spawn_cleanup.config.users {
                info!("[USERS] Checking now for inactive torrents in users.");
                tracker_spawn_cleanup.clean_users_active_torrents(Duration::from_secs(tracker_spawn_cleanup.config.clone().peer_timeout.unwrap())).await;
                info!("[USERS] Inactive torrents in users cleaned up.");
            }
        }
    });

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Shutdown request received, shutting down...");
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
            // let _ = udp_tx.send(true);
            // let _ = udp_futures.into_iter()
            //     .collect::<TryJoinAll<_>>();
            info!("Server shutting down completed");
            Ok(())
        }
    }
}