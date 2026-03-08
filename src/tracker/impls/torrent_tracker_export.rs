use crate::structs::Cli;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use log::{
    error,
    info
};
use std::fs;
use std::process::exit;
use std::sync::Arc;

impl TorrentTracker {
    pub async fn export(&self, args: &Cli, tracker: Arc<TorrentTracker>)
    {
        info!("[EXPORT] Requesting to export data");
        let config = &tracker.config.tracker_config;
        let torrents_file = &args.export_file_torrents;
        info!("[EXPORT] Exporting torrents to file {torrents_file}");
        let torrents_data = serde_json::to_vec(&tracker.torrents_sharding.get_all_content())
            .expect("[EXPORT] Failed to serialize torrents");
        if let Err(error) = fs::write(torrents_file, torrents_data) {
            error!("[EXPORT] The torrents file {torrents_file} could not be generated!");
            panic!("[EXPORT] {error}")
        }
        info!("[EXPORT] The torrents have been exported");
        if config.whitelist_enabled {
            let whitelists_file = &args.export_file_whitelists;
            info!("[EXPORT] Exporting whitelists to file {whitelists_file}");
            let whitelists_data = serde_json::to_vec(&tracker.get_whitelist())
                .expect("[EXPORT] Failed to serialize whitelists");
            if let Err(error) = fs::write(whitelists_file, whitelists_data) {
                error!("[EXPORT] The whitelists file {whitelists_file} could not be generated!");
                panic!("[EXPORT] {error}")
            }
            info!("[EXPORT] The whitelists have been exported");
        }
        if config.blacklist_enabled {
            let blacklists_file = &args.export_file_blacklists;
            info!("[EXPORT] Exporting blacklists to file {blacklists_file}");
            let blacklists_data = serde_json::to_vec(&tracker.get_blacklist())
                .expect("[EXPORT] Failed to serialize blacklists");
            if let Err(error) = fs::write(blacklists_file, blacklists_data) {
                error!("[EXPORT] The blacklists file {blacklists_file} could not be generated!");
                panic!("[EXPORT] {error}")
            }
            info!("[EXPORT] The blacklists have been exported");
        }
        if config.keys_enabled {
            let keys_file = &args.export_file_keys;
            info!("[EXPORT] Exporting keys to file {keys_file}");
            let keys_data = serde_json::to_vec(&tracker.get_keys())
                .expect("[EXPORT] Failed to serialize keys");
            if let Err(error) = fs::write(keys_file, keys_data) {
                error!("[EXPORT] The keys file {keys_file} could not be generated!");
                panic!("[EXPORT] {error}")
            }
            info!("[EXPORT] The keys have been exported");
        }
        if config.users_enabled {
            let users_file = &args.export_file_users;
            info!("[EXPORT] Exporting users to file {users_file}");
            let users_data = serde_json::to_vec(&tracker.get_users())
                .expect("[EXPORT] Failed to serialize users");
            if let Err(error) = fs::write(users_file, users_data) {
                error!("[EXPORT] The users file {users_file} could not be generated!");
                panic!("[EXPORT] {error}")
            }
            info!("[EXPORT] The users have been exported");
        }
        info!("[EXPORT] Exporting of data completed");
        exit(0)
    }
}