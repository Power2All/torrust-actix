use std::sync::atomic::{AtomicBool, AtomicI64};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct StatsAtomics {
    pub started: AtomicI64,
    pub timestamp_run_save: AtomicI64,
    pub timestamp_run_timeout: AtomicI64,
    pub timestamp_run_console: AtomicI64,
    pub timestamp_run_keys_timeout: AtomicI64,
    pub torrents: AtomicI64,
    pub torrents_updates: AtomicI64,
    pub torrents_shadow: AtomicI64,
    pub users: AtomicI64,
    pub users_updates: AtomicI64,
    pub users_shadow: AtomicI64,
    pub maintenance_mode: AtomicI64,
    pub seeds: AtomicI64,
    pub peers: AtomicI64,
    pub completed: AtomicI64,
    pub whitelist_enabled: AtomicBool,
    pub whitelist: AtomicI64,
    pub blacklist_enabled: AtomicBool,
    pub blacklist: AtomicI64,
    pub keys_enabled: AtomicBool,
    pub keys: AtomicI64,
    pub tcp4_connections_handled: AtomicI64,
    pub tcp4_api_handled: AtomicI64,
    pub tcp4_announces_handled: AtomicI64,
    pub tcp4_scrapes_handled: AtomicI64,
    pub tcp6_connections_handled: AtomicI64,
    pub tcp6_api_handled: AtomicI64,
    pub tcp6_announces_handled: AtomicI64,
    pub tcp6_scrapes_handled: AtomicI64,
    pub udp4_connections_handled: AtomicI64,
    pub udp4_announces_handled: AtomicI64,
    pub udp4_scrapes_handled: AtomicI64,
    pub udp6_connections_handled: AtomicI64,
    pub udp6_announces_handled: AtomicI64,
    pub udp6_scrapes_handled: AtomicI64,
    pub test_counter: AtomicI64,
    pub test_counter_udp: AtomicI64,
}
