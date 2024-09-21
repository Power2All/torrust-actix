use std::fs;
use std::process::exit;
use std::sync::Arc;
use log::{error, info};
use serde_json::Value;
use crate::structs::Cli;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

impl TorrentTracker {
    pub async fn import(&self, args: &Cli, tracker: Arc<TorrentTracker>)
    {
        info!("[IMPORT] Requesting to import data");

        info!("[IMPORT] Importing torrents to memory {}", args.import_file_torrents.as_str());
        match fs::read(format!("{}", args.import_file_torrents.as_str())) {
            Ok(data) => {
                let torrents: Value = serde_json::from_str(String::from_utf8(data).unwrap().as_str()).unwrap();
                for (key, value) in torrents.as_object().unwrap() {
                    let completed = match value["completed"].as_u64() {
                        None => { panic!("[IMPORT] 'completed' field doesn't exist or is missing!"); }
                        Some(completed) => { completed }
                    };
                    let info_hash = match hex::decode(key) {
                        Ok(hash_result) => { InfoHash(<[u8; 20]>::try_from(hash_result[0..20].as_ref()).unwrap()) }
                        Err(_) => { panic!("[IMPORT] Torrent hash is not hex or invalid!"); }
                    };
                    tracker.add_torrent_update(info_hash, TorrentEntry {
                        seeds: Default::default(),
                        peers: Default::default(),
                        completed,
                        updated: std::time::Instant::now(),
                    });
                }
                tracker.save_torrent_updates(tracker.clone()).await;
            }
            Err(error) => {
                error!("[IMPORT] The torrents file {} could not be imported!", args.import_file_torrents.as_str());
                panic!("[IMPORT] {}", error.to_string())
            }
        }

        if tracker.config.tracker_config.clone().unwrap().whitelist_enabled.unwrap() {
            info!("[IMPORT] Importing whitelists to memory {}", args.import_file_whitelists.as_str());
            match fs::read(format!("{}", args.import_file_whitelists.as_str())) {
                Ok(data) => {
                    let whitelists: Value = serde_json::from_str(String::from_utf8(data).unwrap().as_str()).unwrap();
                    for value in whitelists.as_array().unwrap() {
                        let info_hash = match hex::decode(value.as_str().unwrap()) {
                            Ok(hash_result) => { InfoHash(<[u8; 20]>::try_from(hash_result[0..20].as_ref()).unwrap()) }
                            Err(_) => { panic!("[IMPORT] Torrent hash is not hex or invalid!"); }
                        };
                        tracker.add_whitelist(info_hash);
                    }
                    match tracker.save_whitelist(tracker.clone(), tracker.get_whitelist()).await {
                        Ok(_) => {}
                        Err(_) => { panic!("[IMPORT] Unable to save whitelist to the database!"); }
                    }
                }
                Err(error) => {
                    error!("[IMPORT] The whitelists file {} could not be imported!", args.import_file_whitelists.as_str());
                    panic!("[IMPORT] {}", error.to_string())
                }
            }
        }

        if tracker.config.tracker_config.clone().unwrap().blacklist_enabled.unwrap() {
            info!("[IMPORT] Importing blacklists to memory {}", args.import_file_blacklists.as_str());
            match fs::read(format!("{}", args.import_file_blacklists.as_str())) {
                Ok(data) => {
                    let blacklists: Value = serde_json::from_str(String::from_utf8(data).unwrap().as_str()).unwrap();
                    for value in blacklists.as_array().unwrap() {
                        let info_hash = match hex::decode(value.as_str().unwrap()) {
                            Ok(hash_result) => { InfoHash(<[u8; 20]>::try_from(hash_result[0..20].as_ref()).unwrap()) }
                            Err(_) => { panic!("[IMPORT] Torrent hash is not hex or invalid!"); }
                        };
                        tracker.add_blacklist(info_hash);
                    }
                    match tracker.save_blacklist(tracker.clone(), tracker.get_blacklist()).await {
                        Ok(_) => {}
                        Err(_) => { panic!("[IMPORT] Unable to save blacklist to the database!"); }
                    }
                }
                Err(error) => {
                    error!("[IMPORT] The blacklists file {} could not be imported!", args.import_file_blacklists.as_str());
                    panic!("[IMPORT] {}", error.to_string())
                }
            }
        }

        if tracker.config.tracker_config.clone().unwrap().keys_enabled.unwrap() {
            info!("[IMPORT] Importing keys to memory {}", args.import_file_keys.as_str());
            match fs::read(format!("{}", args.import_file_keys.as_str())) {
                Ok(data) => {
                    let keys: Value = serde_json::from_str(String::from_utf8(data).unwrap().as_str()).unwrap();
                    for value in keys.as_array().unwrap() {
                        let info_hash = match hex::decode(value.as_str().unwrap()) {
                            Ok(hash_result) => { InfoHash(<[u8; 20]>::try_from(hash_result[0..20].as_ref()).unwrap()) }
                            Err(_) => { panic!("[IMPORT] Torrent hash is not hex or invalid!"); }
                        };
                        tracker.add_blacklist(info_hash);
                    }
                    match tracker.save_blacklist(tracker.clone(), tracker.get_blacklist()).await {
                        Ok(_) => {}
                        Err(_) => { panic!("[IMPORT] Unable to save blacklist to the database!"); }
                    }
                }
                Err(error) => {
                    error!("[IMPORT] The blacklists file {} could not be imported!", args.import_file_blacklists.as_str());
                    panic!("[IMPORT] {}", error.to_string())
                }
            }
        }

        info!("[IMPORT] Importing of data completed");
        exit(0)
    }
}