use std::mem;
use std::net::SocketAddr;
use std::process::exit;
use std::sync::Arc;
use std::time::Duration;
use async_std::task;
use clap::Parser;
use futures_util::future::try_join_all;
use log::{error, info, warn};
use parking_lot::deadlock;
use sentry::ClientInitGuard;
use tokio::runtime::Builder;
use tokio_shutdown::Shutdown;
use torrust_actix::api::api::api_service;
use torrust_actix::common::common::{setup_logging, udp_check_host_and_port_used};
use torrust_actix::config::enums::cluster_mode::ClusterMode;
use torrust_actix::config::structs::configuration::Configuration;
use torrust_actix::http::http::{http_check_host_and_port_used, http_service};
use torrust_actix::structs::Cli;
use torrust_actix::stats::enums::stats_event::StatsEvent;
use torrust_actix::tracker::structs::torrent_tracker::TorrentTracker;
use torrust_actix::udp::udp::udp_service;
use torrust_actix::websocket::master::server::websocket_master_service;
use torrust_actix::websocket::slave::client::start_slave_client;

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
    if config.sentry_config.enabled {
        _sentry_guard = sentry::init((config.sentry_config.dsn.clone(), sentry::ClientOptions {
            release: sentry::release_name!(),
            debug: config.sentry_config.debug,
            sample_rate: config.sentry_config.sample_rate,
            max_breadcrumbs: config.sentry_config.max_breadcrumbs,
            attach_stacktrace: config.sentry_config.attach_stacktrace,
            send_default_pii: config.sentry_config.send_default_pii,
            traces_sample_rate: config.sentry_config.traces_sample_rate,
            session_mode: sentry::SessionMode::Request,
            auto_session_tracking: true,
            ..Default::default()
        }));
    }

    Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let tracker = Arc::new(TorrentTracker::new(config.clone(), args.create_database).await);

            let tracker_config = tracker.config.tracker_config.clone();
            let db_config = tracker.config.database.clone();

            if db_config.persistent {
                tracker.load_torrents(tracker.clone()).await;

                if tracker_config.whitelist_enabled {
                    tracker.load_whitelist(tracker.clone()).await;
                }
                if tracker_config.blacklist_enabled {
                    tracker.load_blacklist(tracker.clone()).await;
                }
                if tracker_config.keys_enabled {
                    tracker.load_keys(tracker.clone()).await;
                }
                if tracker_config.users_enabled {
                    tracker.load_users(tracker.clone()).await;
                }
                if db_config.update_peers && !tracker.reset_seeds_peers(tracker.clone()).await {
                    panic!("[RESET SEEDS PEERS] Unable to continue loading");
                }
            } else {
                tracker.set_stats(StatsEvent::Completed, config.tracker_config.total_downloads as i64);
            }

            if args.create_selfsigned { tracker.cert_gen(&args).await; }
            if args.export { tracker.export(&args, tracker.clone()).await; }
            if args.import { tracker.import(&args, tracker.clone()).await; }

            let tokio_core = Builder::new_multi_thread().thread_name("core").worker_threads(9).enable_all().build()?;
            let tokio_shutdown = Shutdown::new().expect("shutdown creation works on first call");

            let deadlocks_handler = tokio_shutdown.clone();
            tokio_core.spawn(async move {
                info!("[BOOT] Starting thread for deadlocks...");
                let mut interval = tokio::time::interval(Duration::from_secs(30));
                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            let deadlocks = deadlock::check_deadlock();
                            if !deadlocks.is_empty() {
                                info!("[DEADLOCK] Found {} deadlocks", deadlocks.len());
                                for (i, threads) in deadlocks.iter().enumerate() {
                                    info!("[DEADLOCK] #{i}");
                                    for t in threads {
                                        info!("[DEADLOCK] Thread ID: {:#?}", t.thread_id());
                                        info!("[DEADLOCK] {:#?}", t.backtrace());
                                        sentry::capture_message(&format!("{:#?}", t.backtrace()), sentry::Level::Error);
                                    }
                                }
                            }
                        }
                        _ = deadlocks_handler.handle() => {
                            info!("[BOOT] Shutting down thread for deadlocks...");
                            return;
                        }
                    }
                }
            });

            let mut api_futures = Vec::new();
            let mut apis_futures = Vec::new();

            for api_server_object in &config.api_server {
                if api_server_object.enabled {
                    http_check_host_and_port_used(api_server_object.bind_address.clone());
                    let address: SocketAddr = api_server_object.bind_address.parse().unwrap();

                    let (handle, future) = api_service(
                        address,
                        tracker.clone(),
                        api_server_object.clone()
                    ).await;

                    if api_server_object.ssl {
                        apis_futures.push((handle, future));
                    } else {
                        api_futures.push((handle, future));
                    }
                }
            }

            if !api_futures.is_empty() {
                let (handles, futures): (Vec<_>, Vec<_>) = api_futures.into_iter().unzip();
                tokio_core.spawn(async move {
                    let _ = try_join_all(futures).await;
                    drop(handles);
                });
            }
            if !apis_futures.is_empty() {
                let (handles, futures): (Vec<_>, Vec<_>) = apis_futures.into_iter().unzip();
                tokio_core.spawn(async move {
                    let _ = try_join_all(futures).await;
                    drop(handles);
                });
            }

            let mut http_futures = Vec::new();
            let mut https_futures = Vec::new();

            for http_server_object in &config.http_server {
                if http_server_object.enabled {
                    http_check_host_and_port_used(http_server_object.bind_address.clone());
                    let address: SocketAddr = http_server_object.bind_address.parse().unwrap();

                    let (handle, future) = http_service(
                        address,
                        tracker.clone(),
                        http_server_object.clone()
                    ).await;

                    if http_server_object.ssl {
                        https_futures.push((handle, future));
                    } else {
                        http_futures.push((handle, future));
                    }
                }
            }

            if !http_futures.is_empty() {
                let (handles, futures): (Vec<_>, Vec<_>) = http_futures.into_iter().unzip();
                tokio_core.spawn(async move {
                    let _ = try_join_all(futures).await;
                    drop(handles);
                });
            }
            if !https_futures.is_empty() {
                let (handles, futures): (Vec<_>, Vec<_>) = https_futures.into_iter().unzip();
                tokio_core.spawn(async move {
                    let _ = try_join_all(futures).await;
                    drop(handles);
                });
            }

            let (udp_tx, udp_rx) = tokio::sync::watch::channel(false);
            let mut udp_tokio_threads = Vec::new();
            let mut udp_futures = Vec::new();

            for udp_server_object in &config.udp_server {
                if udp_server_object.enabled {
                    udp_check_host_and_port_used(udp_server_object.bind_address.clone());
                    let address: SocketAddr = udp_server_object.bind_address.parse().unwrap();

                    let udp_threads: usize = udp_server_object.udp_threads;
                    let worker_threads: usize = udp_server_object.worker_threads;

                    let tokio_udp = Arc::new(Builder::new_multi_thread()
                        .thread_name("udp")
                        .worker_threads(udp_threads)
                        .enable_all()
                        .build()?);

                    let udp_future = udp_service(
                        address,
                        udp_threads,
                        worker_threads,
                        udp_server_object.receive_buffer_size,
                        udp_server_object.send_buffer_size,
                        udp_server_object.reuse_address,
                        udp_server_object.use_payload_ip,
                        tracker.clone(),
                        udp_rx.clone(),
                        tokio_udp.clone()
                    ).await;

                    udp_futures.push(udp_future);
                    udp_tokio_threads.push(tokio_udp);
                }
            }

            
            let cluster_mode = tracker_config.cluster.clone();
            let mut ws_futures = Vec::new();

            match cluster_mode {
                ClusterMode::master => {
                    
                    let bind_address = &tracker_config.cluster_bind_address;
                    if !bind_address.is_empty() {
                        http_check_host_and_port_used(bind_address.clone());
                        let address: SocketAddr = bind_address.parse().expect("Invalid cluster_bind_address");

                        info!("[CLUSTER] Starting WebSocket master server on {}", address);

                        let (handle, future) = websocket_master_service(
                            address,
                            tracker.clone(),
                        ).await;

                        ws_futures.push((handle, future));
                    } else {
                        warn!("[CLUSTER] Master mode enabled but cluster_bind_address is empty");
                    }
                }
                ClusterMode::slave => {
                    
                    let master_address = &tracker_config.cluster_master_address;
                    if !master_address.is_empty() {
                        info!("[CLUSTER] Starting WebSocket slave client connecting to {}", master_address);

                        
                        let tracker_slave = tracker.clone();
                        let shutdown_handler = tokio_shutdown.clone();
                        tokio_core.spawn(async move {
                            tokio::select! {
                                _ = start_slave_client(tracker_slave) => {
                                    info!("[CLUSTER] Slave client stopped");
                                }
                                _ = shutdown_handler.handle() => {
                                    info!("[CLUSTER] Shutting down slave client...");
                                }
                            }
                        });
                    } else {
                        error!("[CLUSTER] Slave mode enabled but cluster_master_address is empty");
                        exit(1);
                    }
                }
                ClusterMode::standalone => {
                    
                    info!("[CLUSTER] Running in standalone mode");
                }
            }

            
            if !ws_futures.is_empty() {
                let (handles, futures): (Vec<_>, Vec<_>) = ws_futures.into_iter().unzip();
                tokio_core.spawn(async move {
                    let _ = try_join_all(futures).await;
                    drop(handles);
                });
            }

            let stats_handler = tokio_shutdown.clone();
            let tracker_spawn_stats = tracker.clone();
            let console_interval = tracker_spawn_stats.config.log_console_interval;
            info!("[BOOT] Starting thread for console updates with {console_interval} seconds delay...");

            tokio_core.spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(console_interval));
                
                let mut last_udp: Option<(i64,i64,i64,i64,i64,i64,i64)> = None; 
                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            tracker_spawn_stats.set_stats(StatsEvent::TimestampSave, chrono::Utc::now().timestamp() + 60);
                            let stats = tracker_spawn_stats.get_stats();

                            info!(
                                "[STATS] Torrents: {} - Updates: {} - Seeds: {} - Peers: {} - Completed: {} | \
                                WList: {} - WList Updates: {} - BLists: {} - BLists Updates: {} - Keys: {} - Keys Updates {}",
                                stats.torrents, stats.torrents_updates, stats.seeds, stats.peers, stats.completed,
                                stats.whitelist, stats.whitelist_updates, stats.blacklist, stats.blacklist_updates,
                                stats.keys, stats.keys_updates
                            );

                            info!(
                                "[STATS TCP] IPv4: Conn:{} API:{} A:{} S:{} F:{} 404:{} | IPv6: Conn:{} API:{} A:{} S:{} F:{} 404:{}",
                                stats.tcp4_connections_handled, stats.tcp4_api_handled, stats.tcp4_announces_handled,
                                stats.tcp4_scrapes_handled, stats.tcp4_failure, stats.tcp4_not_found,
                                stats.tcp6_connections_handled, stats.tcp6_api_handled, stats.tcp6_announces_handled,
                                stats.tcp6_scrapes_handled, stats.tcp6_failure, stats.tcp6_not_found
                            );

                            
                            let now = chrono::Utc::now().timestamp();
                            let (udp_c4_ps, udp_a4_ps, udp_s4_ps, udp_c6_ps, udp_a6_ps, udp_s6_ps) = if let Some((t,c4,a4,s4,c6,a6,s6)) = last_udp {
                                let dt = (now - t).max(1);
                                (
                                    (stats.udp4_connections_handled - c4)/dt,
                                    (stats.udp4_announces_handled - a4)/dt,
                                    (stats.udp4_scrapes_handled - s4)/dt,
                                    (stats.udp6_connections_handled - c6)/dt,
                                    (stats.udp6_announces_handled - a6)/dt,
                                    (stats.udp6_scrapes_handled - s6)/dt,
                                )
                            } else { (0,0,0,0,0,0) };
                            last_udp = Some((now, stats.udp4_connections_handled, stats.udp4_announces_handled, stats.udp4_scrapes_handled,
                                             stats.udp6_connections_handled, stats.udp6_announces_handled, stats.udp6_scrapes_handled));

                            info!(
                                "[STATS UDP] IPv4: Conn:{} ({}s) A:{} ({}s) S:{} ({}s) IR:{} BR:{} | IPv6: Conn:{} ({}s) A:{} ({}s) S:{} ({}s) IR:{} BR:{} | Q:{}",
                                stats.udp4_connections_handled, udp_c4_ps, stats.udp4_announces_handled, udp_a4_ps, stats.udp4_scrapes_handled, udp_s4_ps,
                                stats.udp4_invalid_request, stats.udp4_bad_request,
                                stats.udp6_connections_handled, udp_c6_ps, stats.udp6_announces_handled, udp_a6_ps, stats.udp6_scrapes_handled, udp_s6_ps,
                                stats.udp6_invalid_request, stats.udp6_bad_request,
                                stats.udp_queue_len
                            );

                            
                            if tracker_spawn_stats.config.tracker_config.cluster != ClusterMode::standalone {
                                info!(
                                    "[STATS WS] Conn:{} | Req: Sent:{} Recv:{} | Resp: Sent:{} Recv:{} | TO:{} Recon:{} | Auth: OK:{} Fail:{}",
                                    stats.ws_connections_active,
                                    stats.ws_requests_sent, stats.ws_requests_received,
                                    stats.ws_responses_sent, stats.ws_responses_received,
                                    stats.ws_timeouts, stats.ws_reconnects,
                                    stats.ws_auth_success, stats.ws_auth_failed
                                );
                            }
                        }
                        _ = stats_handler.handle() => {
                            info!("[BOOT] Shutting down thread for console updates...");
                            return;
                        }
                    }
                }
            });

            let tracker_cleanup_clone = tracker.clone();
            let cleanup_handler = tokio_shutdown.clone();
            let cleanup_interval = tracker_cleanup_clone.config.tracker_config.peers_cleanup_interval;
            info!("[BOOT] Starting thread for peers cleanup with {cleanup_interval} seconds delay...");

            let peers_timeout = tracker_cleanup_clone.config.tracker_config.peers_timeout;
            let persistent = tracker_cleanup_clone.config.database.persistent;
            let torrents_sharding = tracker_cleanup_clone.torrents_sharding.clone();

            tokio_core.spawn(async move {
                torrents_sharding.cleanup_threads(
                    tracker_cleanup_clone,
                    cleanup_handler,
                    Duration::from_secs(peers_timeout),
                    persistent
                ).await;
            });

            if tracker_config.keys_enabled {
                let cleanup_keys_handler = tokio_shutdown.clone();
                let tracker_spawn_cleanup_keys = tracker.clone();
                let keys_interval = tracker_spawn_cleanup_keys.config.tracker_config.keys_cleanup_interval;
                info!("[BOOT] Starting thread for keys cleanup with {keys_interval} seconds delay...");

                tokio_core.spawn(async move {
                    let mut interval = tokio::time::interval(Duration::from_secs(keys_interval));
                    loop {
                        tokio::select! {
                            _ = interval.tick() => {
                                tracker_spawn_cleanup_keys.set_stats(StatsEvent::TimestampKeysTimeout,
                                    chrono::Utc::now().timestamp() + keys_interval as i64);
                                info!("[KEYS] Checking now for outdated keys.");
                                tracker_spawn_cleanup_keys.clean_keys();
                                info!("[KEYS] Keys cleaned up.");
                            }
                            _ = cleanup_keys_handler.handle() => {
                                info!("[BOOT] Shutting down thread for keys cleanup...");
                                return;
                            }
                        }
                    }
                });
            }

            if db_config.persistent {
                let updates_handler = tokio_shutdown.clone();
                let tracker_spawn_updates = tracker.clone();
                let update_interval = tracker_spawn_updates.config.database.persistent_interval;
                info!("[BOOT] Starting thread for database updates with {update_interval} seconds delay...");

                tokio_core.spawn(async move {
                    let mut interval = tokio::time::interval(Duration::from_secs(update_interval));
                    loop {
                        tokio::select! {
                            _ = interval.tick() => {
                                tracker_spawn_updates.set_stats(StatsEvent::TimestampSave,
                                    chrono::Utc::now().timestamp() + update_interval as i64);

                                info!("[DATABASE UPDATES] Starting batch updates...");

                                
                                let mut tasks = vec![
                                    tokio::spawn({
                                        let tracker = tracker_spawn_updates.clone();
                                        async move {
                                            let _ = tracker.save_torrent_updates(tracker.clone()).await;
                                        }
                                    })
                                ];

                                if tracker_spawn_updates.config.tracker_config.whitelist_enabled {
                                    tasks.push(tokio::spawn({
                                        let tracker = tracker_spawn_updates.clone();
                                        async move {
                                            let _ = tracker.save_whitelist_updates(tracker.clone()).await;
                                        }
                                    }));
                                }

                                if tracker_spawn_updates.config.tracker_config.blacklist_enabled {
                                    tasks.push(tokio::spawn({
                                        let tracker = tracker_spawn_updates.clone();
                                        async move {
                                            let _ = tracker.save_blacklist_updates(tracker.clone()).await;
                                        }
                                    }));
                                }

                                if tracker_spawn_updates.config.tracker_config.keys_enabled {
                                    tasks.push(tokio::spawn({
                                        let tracker = tracker_spawn_updates.clone();
                                        async move {
                                            let _ = tracker.save_key_updates(tracker.clone()).await;
                                        }
                                    }));
                                }

                                if tracker_spawn_updates.config.tracker_config.users_enabled {
                                    tasks.push(tokio::spawn({
                                        let tracker = tracker_spawn_updates.clone();
                                        async move {
                                            let _ = tracker.save_user_updates(tracker.clone()).await;
                                        }
                                    }));
                                }

                                
                                for task in tasks {
                                    let _ = task.await;
                                }

                                info!("[DATABASE UPDATES] Batch updates completed");
                            }
                            _ = updates_handler.handle() => {
                                info!("[BOOT] Shutting down thread for updates...");
                                return;
                            }
                        }
                    }
                });
            }

            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    info!("Shutdown request received, shutting down...");

                    let _ = udp_tx.send(true);

                    match try_join_all(udp_futures).await {
                        Ok(_) => {}
                        Err(error) => {
                            sentry::capture_error(&error);
                            error!("Errors happened on shutting down UDP sockets: {error}");
                        }
                    }

                    tokio_shutdown.handle().await;
                    task::sleep(Duration::from_secs(1)).await;

                    if db_config.persistent {
                        tracker.set_stats(StatsEvent::Completed, config.tracker_config.total_downloads as i64);
                        Configuration::save_from_config(tracker.config.clone(), "config.toml");

                        info!("Saving final data to database...");

                        
                        let mut tasks = vec![
                            tokio::spawn({
                                let tracker_clone = tracker.clone();
                                async move {
                                    let _ = tracker_clone.save_torrent_updates(tracker_clone.clone()).await;
                                }
                            })
                        ];

                        if tracker_config.whitelist_enabled {
                            tasks.push(tokio::spawn({
                                let tracker_clone = tracker.clone();
                                async move {
                                    let _ = tracker_clone.save_whitelist_updates(tracker_clone.clone()).await;
                                }
                            }));
                        }

                        if tracker_config.blacklist_enabled {
                            tasks.push(tokio::spawn({
                                let tracker_clone = tracker.clone();
                                async move {
                                    let _ = tracker_clone.save_blacklist_updates(tracker_clone.clone()).await;
                                }
                            }));
                        }

                        if tracker_config.keys_enabled {
                            tasks.push(tokio::spawn({
                                let tracker_clone = tracker.clone();
                                async move {
                                    let _ = tracker_clone.save_key_updates(tracker_clone.clone()).await;
                                }
                            }));
                        }

                        if tracker_config.users_enabled {
                            tasks.push(tokio::spawn({
                                let tracker_clone = tracker.clone();
                                async move {
                                    let _ = tracker_clone.save_user_updates(tracker_clone.clone()).await;
                                }
                            }));
                        }

                        
                        for task in tasks {
                            let _ = task.await;
                        }
                    } else {
                        tracker.set_stats(StatsEvent::Completed, config.tracker_config.total_downloads as i64);
                        Configuration::save_from_config(tracker.config.clone(), "config.toml");
                        info!("Saving completed data to config...");
                    }

                    task::sleep(Duration::from_secs(1)).await;
                    info!("Server shutting down completed");

                    mem::forget(tokio_core);
                    mem::forget(udp_tokio_threads);
                    Ok(())
                }
            }
        })
}