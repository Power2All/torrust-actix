use std::net::SocketAddr;
use std::process::exit;
use std::sync::Arc;
use std::time::Duration;
use async_std::task;
use clap::Parser;
use futures_util::future::{try_join_all, TryJoinAll};
use log::{error, info};
use parking_lot::deadlock;
use sentry::ClientInitGuard;
use tokio_shutdown::Shutdown;
use torrust_actix::api::api::api_service;
use torrust_actix::common::common::{setup_logging, shutdown_waiting, udp_check_host_and_port_used};
use torrust_actix::config::structs::configuration::Configuration;
use torrust_actix::http::http::{http_check_host_and_port_used, http_service};
use torrust_actix::structs::Cli;
use torrust_actix::stats::enums::stats_event::StatsEvent;
use torrust_actix::tracker::structs::torrent_tracker::TorrentTracker;
use torrust_actix::udp::udp::udp_service;

#[tracing::instrument(level = "debug")]
fn main() -> std::io::Result<()>
{
    let args = Cli::parse();

    let config = match Configuration::load_from_file(args.create_config) {
        Ok(config) => Arc::new(config),
        Err(_) => exit(101)
    };

    setup_logging(&config);

    info!("{} - Version: {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    #[warn(unused_variables)]
    let _sentry_guard: ClientInitGuard;
    if config.sentry_config.clone().enabled {
        _sentry_guard = sentry::init((config.sentry_config.clone().dsn, sentry::ClientOptions {
            release: sentry::release_name!(),
            debug: config.sentry_config.clone().debug,
            sample_rate: config.sentry_config.clone().sample_rate,
            max_breadcrumbs: config.sentry_config.clone().max_breadcrumbs,
            attach_stacktrace: config.sentry_config.clone().attach_stacktrace,
            send_default_pii: config.sentry_config.clone().send_default_pii,
            traces_sample_rate: config.sentry_config.clone().traces_sample_rate,
            session_mode: sentry::SessionMode::Request,
            auto_session_tracking: true,
            ..Default::default()
        }));
    }

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let tracker = Arc::new(TorrentTracker::new(config.clone(), args.create_database).await);

            if tracker.config.database.clone().persistent {
                tracker.load_torrents(tracker.clone()).await;
                if tracker.config.tracker_config.clone().whitelist_enabled {
                    tracker.load_whitelist(tracker.clone()).await;
                }
                if tracker.config.tracker_config.clone().blacklist_enabled {
                    tracker.load_blacklist(tracker.clone()).await;
                }
                if tracker.config.tracker_config.clone().keys_enabled {
                    tracker.load_keys(tracker.clone()).await;
                }
                if tracker.config.tracker_config.clone().users_enabled {
                    tracker.load_users(tracker.clone()).await;
                }
                if tracker.config.database.clone().update_peers && !tracker.reset_seeds_peers(tracker.clone()).await {
                    panic!("[RESET SEEDS PEERS] Unable to continue loading");
                }
            } else {
                tracker.set_stats(StatsEvent::Completed, config.tracker_config.clone().total_downloads as i64);
            }

            if args.create_selfsigned { tracker.cert_gen(&args).await; }

            if args.export { tracker.export(&args, tracker.clone()).await; }

            if args.import { tracker.import(&args, tracker.clone()).await; }

            let tokio_shutdown = Shutdown::new().expect("shutdown creation works on first call");

            let deadlocks_handler = tokio_shutdown.clone();
            tokio::spawn(async move {
                info!("[BOOT] Starting thread for deadlocks...");
                loop {
                    if shutdown_waiting(Duration::from_secs(10), deadlocks_handler.clone()).await {
                        info!("[BOOT] Shutting down thread for deadlocks...");
                        return;
                    }
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
                            sentry::capture_message(format!("{:#?}", t.backtrace()).as_str(), sentry::Level::Error);
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
                            api_server_object.clone()
                        ).await;
                        apis_handlers.push(handle);
                        apis_futures.push(https_api);
                    } else {
                        let (handle, http_api) = api_service(
                            address,
                            tracker.clone(),
                            api_server_object.clone()
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
                            http_server_object.clone()
                        ).await;
                        https_handlers.push(handle);
                        https_futures.push(https_service);
                    } else {
                        let (handle, http_service) = http_service(
                            address,
                            tracker.clone(),
                            http_server_object.clone()
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

            let (udp_tx, udp_rx) = tokio::sync::watch::channel(false);
            let mut udp_futures = Vec::new();
            for udp_server_object in &config.udp_server {
                if udp_server_object.enabled {
                    udp_check_host_and_port_used(udp_server_object.bind_address.clone());
                    let address: SocketAddr = udp_server_object.bind_address.parse().unwrap();
                    let threads: u64 = udp_server_object.threads;
                    let tracker_clone = tracker.clone();
                    udp_futures.push(udp_service(address, threads, tracker_clone, udp_rx.clone()).await);
                }
            }

            let stats_handler = tokio_shutdown.clone();
            let tracker_spawn_stats = tracker.clone();
            info!("[BOOT] Starting thread for console updates with {} seconds delay...", tracker_spawn_stats.config.log_console_interval);
            tokio::spawn(async move {
                loop {
                    tracker_spawn_stats.set_stats(StatsEvent::TimestampSave, chrono::Utc::now().timestamp() + 60i64);
                    if shutdown_waiting(Duration::from_secs(tracker_spawn_stats.config.log_console_interval), stats_handler.clone()).await {
                        info!("[BOOT] Shutting down thread for console updates...");
                        return;
                    }

                    let stats = tracker_spawn_stats.get_stats();
                    info!("[STATS] Torrents: {} - Updates: {} - Seeds: {} - Peers: {} - Completed: {}", stats.torrents, stats.torrents_updates, stats.seeds, stats.peers, stats.completed);
                    info!("[STATS] WList: {} - WList Updates: {} - BLists: {} - BLists Updates: {} - Keys: {} - Keys Updates {}", stats.whitelist, stats.whitelist_updates, stats.blacklist, stats.blacklist_updates, stats.keys, stats.keys_updates);
                    info!("[STATS TCP IPv4] Connect: {} - API: {} - A: {} - S: {} - F: {} - 404: {}", stats.tcp4_connections_handled, stats.tcp4_api_handled, stats.tcp4_announces_handled, stats.tcp4_scrapes_handled, stats.tcp4_failure, stats.tcp4_not_found);
                    info!("[STATS TCP IPv6] Connect: {} - API: {} - A: {} - S: {} - F: {} - 404: {}", stats.tcp6_connections_handled, stats.tcp6_api_handled, stats.tcp6_announces_handled, stats.tcp6_scrapes_handled, stats.tcp6_failure, stats.tcp6_not_found);
                    info!("[STATS UDP IPv4] Connect: {} - A: {} - S: {} - IR: {} - BR: {}", stats.udp4_connections_handled, stats.udp4_announces_handled, stats.udp4_scrapes_handled, stats.udp4_invalid_request, stats.udp4_bad_request);
                    info!("[STATS UDP IPv6] Connect: {} - A: {} - S: {} - IR: {} - BR: {}", stats.udp6_connections_handled, stats.udp6_announces_handled, stats.udp6_scrapes_handled, stats.udp6_invalid_request, stats.udp6_bad_request);
                }
            });

            let tracker_spawn_cleanup_peers = tracker.clone();
            info!("[BOOT] Starting thread for peers cleanup with {} seconds delay...", tracker_spawn_cleanup_peers.config.tracker_config.clone().peers_cleanup_interval);
            tracker.clone().torrents_sharding.cleanup_threads(tracker.clone(), tokio_shutdown.clone(), Duration::from_secs(tracker_spawn_cleanup_peers.config.tracker_config.clone().peers_timeout), tracker_spawn_cleanup_peers.config.database.clone().persistent);

            if tracker.config.tracker_config.clone().keys_enabled {
                let cleanup_keys_handler = tokio_shutdown.clone();
                let tracker_spawn_cleanup_keys = tracker.clone();
                info!("[BOOT] Starting thread for keys cleanup with {} seconds delay...", tracker_spawn_cleanup_keys.config.tracker_config.clone().keys_cleanup_interval);
                tokio::spawn(async move {
                    loop {
                        tracker_spawn_cleanup_keys.set_stats(StatsEvent::TimestampKeysTimeout, chrono::Utc::now().timestamp() + tracker_spawn_cleanup_keys.config.tracker_config.clone().keys_cleanup_interval as i64);
                        if shutdown_waiting(Duration::from_secs(tracker_spawn_cleanup_keys.config.tracker_config.clone().keys_cleanup_interval), cleanup_keys_handler.clone()).await {
                            info!("[BOOT] Shutting down thread for keys cleanup...");
                            return;
                        }

                        info!("[KEYS] Checking now for outdated keys.");
                        tracker_spawn_cleanup_keys.clean_keys();
                        info!("[KEYS] Keys cleaned up.");
                    }
                });
            }

            if tracker.config.database.clone().persistent {
                let updates_handler = tokio_shutdown.clone();
                let tracker_spawn_updates = tracker.clone();
                info!("[BOOT] Starting thread for database updates with {} seconds delay...", tracker_spawn_updates.config.database.clone().persistent_interval);
                tokio::spawn(async move {
                    loop {
                        tracker_spawn_updates.set_stats(StatsEvent::TimestampSave, chrono::Utc::now().timestamp() + tracker_spawn_updates.config.database.clone().persistent_interval as i64);
                        if shutdown_waiting(Duration::from_secs(tracker_spawn_updates.config.database.clone().persistent_interval), updates_handler.clone()).await {
                            info!("[BOOT] Shutting down thread for updates...");
                            return;
                        }

                        info!("[TORRENTS UPDATES] Start updating torrents into the DB.");
                        let _ = tracker_spawn_updates.save_torrent_updates(tracker_spawn_updates.clone()).await;
                        info!("[TORRENTS UPDATES] Torrent updates inserted into DB.");

                        if tracker_spawn_updates.config.tracker_config.clone().whitelist_enabled {
                            info!("[WHITELIST UPDATES] Start updating whitelists into the DB.");
                            let _ = tracker_spawn_updates.save_whitelist_updates(tracker_spawn_updates.clone()).await;
                            info!("[WHITELIST UPDATES] Whitelists updates inserted into DB.");
                        }

                        if tracker_spawn_updates.config.tracker_config.clone().blacklist_enabled {
                            info!("[BLACKLIST UPDATES] Start updating blacklists into the DB.");
                            let _ = tracker_spawn_updates.save_blacklist_updates(tracker_spawn_updates.clone()).await;
                            info!("[BLACKLIST UPDATES] Blacklists updates inserted into DB.");
                        }

                        if tracker_spawn_updates.config.tracker_config.clone().keys_enabled {
                            info!("[KEY UPDATES] Start updating keys into the DB.");
                            let _ = tracker_spawn_updates.save_key_updates(tracker_spawn_updates.clone()).await;
                            info!("[KEY UPDATES] Keys updates inserted into DB.");
                        }

                        if tracker_spawn_updates.config.tracker_config.clone().users_enabled {
                            info!("[USERS UPDATES] Start updating users into the DB.");
                            let _ = tracker_spawn_updates.save_user_updates(tracker_spawn_updates.clone()).await;
                            info!("[USERS UPDATES] Keys updates inserted into DB.");
                        }
                    }
                });
            }

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
                    let _ = udp_tx.send(true);
                    match udp_futures.into_iter().collect::<TryJoinAll<_>>().await {
                        Ok(_) => {}
                        Err(error) => {
                            sentry::capture_error(&error);
                            error!("Errors happened on shutting down UDP sockets!");
                            error!("{}", error.to_string());
                        }
                    }
                    tokio_shutdown.handle().await;

                    task::sleep(Duration::from_secs(1)).await;

                    if tracker.config.database.clone().persistent {
                        tracker.set_stats(StatsEvent::Completed, config.tracker_config.clone().total_downloads as i64);
                        Configuration::save_from_config(tracker.config.clone(), "config.toml");
                        info!("Saving completed data to an INI...");
                        info!("Saving data to the database...");
                        let _ = tracker.save_torrent_updates(tracker.clone()).await;
                        if tracker.config.tracker_config.clone().whitelist_enabled {
                            let _ = tracker.save_whitelist_updates(tracker.clone()).await;
                        }
                        if tracker.config.tracker_config.clone().blacklist_enabled {
                            let _ = tracker.save_blacklist_updates(tracker.clone()).await;
                        }
                        if tracker.config.tracker_config.clone().keys_enabled {
                            let _ = tracker.save_key_updates(tracker.clone()).await;
                        }
                        if tracker.config.tracker_config.clone().users_enabled {
                            let _ = tracker.save_user_updates(tracker.clone()).await;
                        }
                    } else {
                        tracker.set_stats(StatsEvent::Completed, config.tracker_config.clone().total_downloads as i64);
                        Configuration::save_from_config(tracker.config.clone(), "config.toml");
                        info!("Saving completed data to an INI...");
                    }

                    task::sleep(Duration::from_secs(1)).await;

                    info!("Server shutting down completed");
                    Ok(())
                }
            }
        })
}
