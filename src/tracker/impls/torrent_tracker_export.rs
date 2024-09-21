use std::fs;
use std::process::exit;
use std::sync::Arc;
use log::{error, info};
use crate::structs::Cli;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

impl TorrentTracker {
    pub async fn export(&self, args: &Cli, tracker: Arc<TorrentTracker>)
    {
        info!("[EXPORT] Requesting to export data");

        info!("[EXPORT] Exporting torrents to file {}", args.export_file_torrents.as_str());
        match fs::write(format!("{}", args.export_file_torrents.as_str()), &serde_json::to_vec(&tracker.clone().torrents_sharding.get_all_content()).unwrap()) {
            Ok(_) => {
                info!("[EXPORT] The torrents have been exported");
            }
            Err(error) => {
                error!("[EXPORT] The torrents file {} could not be generated!", args.export_file_torrents.as_str());
                panic!("[EXPORT] {}", error.to_string())
            }
        }

        if tracker.config.tracker_config.clone().unwrap().whitelist_enabled.unwrap() {
            info!("[EXPORT] Exporting whitelists to file {}", args.export_file_whitelists.as_str());
            match fs::write(format!("{}", args.export_file_whitelists.as_str()), &serde_json::to_vec(&tracker.clone().get_whitelist()).unwrap()) {
                Ok(_) => {
                    info!("[EXPORT] The whitelists have been exported");
                }
                Err(error) => {
                    error!("[EXPORT] The whitelists file {} could not be generated!", args.export_file_whitelists.as_str());
                    panic!("[EXPORT] {}", error.to_string())
                }
            }
        }

        if tracker.config.tracker_config.clone().unwrap().blacklist_enabled.unwrap() {
            info!("[EXPORT] Exporting blacklists to file {}", args.export_file_blacklists.as_str());
            match fs::write(format!("{}", args.export_file_blacklists.as_str()), &serde_json::to_vec(&tracker.clone().get_blacklist()).unwrap()) {
                Ok(_) => {
                    info!("[EXPORT] The blacklists have been exported");
                }
                Err(error) => {
                    error!("[EXPORT] The blacklists file {} could not be generated!", args.export_file_blacklists.as_str());
                    panic!("[EXPORT] {}", error.to_string())
                }
            }
        }

        if tracker.config.tracker_config.clone().unwrap().keys_enabled.unwrap() {
            info!("[EXPORT] Exporting keys to file {}", args.export_file_keys.as_str());
            match fs::write(format!("{}", args.export_file_keys.as_str()), &serde_json::to_vec(&tracker.clone().get_keys()).unwrap()) {
                Ok(_) => {
                    info!("[EXPORT] The keys have been exported");
                }
                Err(error) => {
                    error!("[EXPORT] The keys file {} could not be generated!", args.export_file_keys.as_str());
                    panic!("[EXPORT] {}", error.to_string())
                }
            }
        }

        if tracker.config.tracker_config.clone().unwrap().users_enabled.unwrap() {
            info!("[EXPORT] Exporting users to file {}", args.export_file_users.as_str());
            match fs::write(format!("{}", args.export_file_users.as_str()), &serde_json::to_vec(&tracker.clone().get_users()).unwrap()) {
                Ok(_) => {
                    info!("[EXPORT] The users have been exported");
                }
                Err(error) => {
                    error!("[EXPORT] The users file {} could not be generated!", args.export_file_users.as_str());
                    panic!("[EXPORT] {}", error.to_string())
                }
            }
        }

        info!("[EXPORT] Exporting of data completed");
        exit(0)
    }
}