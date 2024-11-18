use std::collections::BTreeMap;
use std::fs;
use std::process::exit;
use std::sync::Arc;
use log::{error, info};
use serde_json::Value;
use crate::structs::Cli;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::tracker::structs::user_entry_item::UserEntryItem;
use crate::tracker::structs::user_id::UserId;

impl TorrentTracker {
    #[tracing::instrument]
    pub async fn import(&self, args: &Cli, tracker: Arc<TorrentTracker>)
    {
        info!("[IMPORT] Requesting to import data");

        info!("[IMPORT] Importing torrents to memory {}", args.import_file_torrents.as_str());
        match fs::read(args.import_file_torrents.as_str()) {
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
                    let _ = tracker.add_torrent_update(info_hash, TorrentEntry {
                        seeds: Default::default(),
                        peers: Default::default(),
                        completed,
                        updated: std::time::Instant::now(),
                    }, UpdatesAction::Add);
                }
                match tracker.save_torrent_updates(tracker.clone()).await {
                    Ok(_) => {}
                    Err(_) => {
                        panic!("[IMPORT] Unable to save torrents to the database!");
                    }
                }
            }
            Err(error) => {
                error!("[IMPORT] The torrents file {} could not be imported!", args.import_file_torrents.as_str());
                panic!("[IMPORT] {}", error)
            }
        }

        if tracker.config.tracker_config.clone().whitelist_enabled {
            info!("[IMPORT] Importing whitelists to memory {}", args.import_file_whitelists.as_str());
            match fs::read(args.import_file_whitelists.as_str()) {
                Ok(data) => {
                    let whitelists: Value = serde_json::from_str(String::from_utf8(data).unwrap().as_str()).unwrap();
                    for value in whitelists.as_array().unwrap() {
                        let info_hash = match hex::decode(value.as_str().unwrap()) {
                            Ok(hash_result) => { InfoHash(<[u8; 20]>::try_from(hash_result[0..20].as_ref()).unwrap()) }
                            Err(_) => { panic!("[IMPORT] Torrent hash is not hex or invalid!"); }
                        };
                        tracker.add_whitelist_update(info_hash, UpdatesAction::Add);
                    }
                    match tracker.save_whitelist_updates(tracker.clone()).await {
                        Ok(_) => {}
                        Err(_) => {
                            panic!("[IMPORT] Unable to save whitelist to the database!");
                        }
                    }
                }
                Err(error) => {
                    error!("[IMPORT] The whitelists file {} could not be imported!", args.import_file_whitelists.as_str());
                    panic!("[IMPORT] {}", error)
                }
            }
        }

        if tracker.config.tracker_config.clone().blacklist_enabled {
            info!("[IMPORT] Importing blacklists to memory {}", args.import_file_blacklists.as_str());
            match fs::read(args.import_file_blacklists.as_str()) {
                Ok(data) => {
                    let blacklists: Value = serde_json::from_str(String::from_utf8(data).unwrap().as_str()).unwrap();
                    for value in blacklists.as_array().unwrap() {
                        let info_hash = match hex::decode(value.as_str().unwrap()) {
                            Ok(hash_result) => { InfoHash(<[u8; 20]>::try_from(hash_result[0..20].as_ref()).unwrap()) }
                            Err(_) => { panic!("[IMPORT] Torrent hash is not hex or invalid!"); }
                        };
                        tracker.add_blacklist_update(info_hash, UpdatesAction::Add);
                    }
                    match tracker.save_blacklist_updates(tracker.clone()).await {
                        Ok(_) => {}
                        Err(_) => { panic!("[IMPORT] Unable to save blacklist to the database!"); }
                    }
                }
                Err(error) => {
                    error!("[IMPORT] The blacklists file {} could not be imported!", args.import_file_blacklists.as_str());
                    panic!("[IMPORT] {}", error)
                }
            }
        }

        if tracker.config.tracker_config.clone().keys_enabled {
            info!("[IMPORT] Importing keys to memory {}", args.import_file_keys.as_str());
            match fs::read(args.import_file_keys.as_str()) {
                Ok(data) => {
                    let keys: Value = serde_json::from_str(String::from_utf8(data).unwrap().as_str()).unwrap();
                    for (key, value) in keys.as_object().unwrap() {
                        let timeout = match value.as_i64() {
                            None => { panic!("[IMPORT] timeout value doesn't exist or is missing!"); }
                            Some(timeout) => { timeout }
                        };
                        let hash = match hex::decode(key.as_str()) {
                            Ok(hash_result) => { InfoHash(<[u8; 20]>::try_from(hash_result[0..20].as_ref()).unwrap()) }
                            Err(_) => { panic!("[IMPORT] Key hash is not hex or invalid!"); }
                        };
                        tracker.add_key_update(hash, timeout, UpdatesAction::Add);
                    }
                    match tracker.save_key_updates(tracker.clone()).await {
                        Ok(_) => {}
                        Err(_) => { panic!("[IMPORT] Unable to save keys to the database!"); }
                    }
                }
                Err(error) => {
                    error!("[IMPORT] The keys file {} could not be imported!", args.import_file_keys.as_str());
                    panic!("[IMPORT] {}", error)
                }
            }
        }

        if tracker.config.tracker_config.clone().users_enabled {
            info!("[IMPORT] Importing users to memory {}", args.import_file_users.as_str());
            match fs::read(args.import_file_users.as_str()) {
                Ok(data) => {
                    let users: Value = serde_json::from_str(String::from_utf8(data).unwrap().as_str()).unwrap();
                    for (key, value) in users.as_object().unwrap() {
                        let user_hash = match hex::decode(key.as_str()) {
                            Ok(hash_result) => { UserId(<[u8; 20]>::try_from(hash_result[0..20].as_ref()).unwrap()) }
                            Err(_) => { panic!("[IMPORT] User hash is not hex or invalid!"); }
                        };
                        let key_hash = match hex::decode(value["key"].as_str().unwrap()) {
                            Ok(hash_result) => { UserId(<[u8; 20]>::try_from(hash_result[0..20].as_ref()).unwrap()) }
                            Err(_) => { panic!("[IMPORT] Key hash is not hex or invalid!"); }
                        };
                        let user_id = value["user_id"].as_u64();
                        let user_uuid = value["user_uuid"].as_str().map(String::from);
                        let uploaded = match value["uploaded"].as_u64() {
                            None => { panic!("[IMPORT] 'uploaded' field doesn't exist or is missing!"); }
                            Some(uploaded) => { uploaded }
                        };
                        let downloaded = match value["downloaded"].as_u64() {
                            None => { panic!("[IMPORT] 'downloaded' field doesn't exist or is missing!"); }
                            Some(downloaded) => { downloaded }
                        };
                        let completed = match value["completed"].as_u64() {
                            None => { panic!("[IMPORT] 'completed' field doesn't exist or is missing!"); }
                            Some(completed) => { completed }
                        };
                        let updated = match value["updated"].as_u64() {
                            None => { panic!("[IMPORT] 'updated' field doesn't exist or is missing!"); }
                            Some(updated) => { updated }
                        };
                        let active = match value["active"].as_u64() {
                            None => { panic!("[IMPORT] 'active' field doesn't exist or is missing!"); }
                            Some(active) => { active as u8 }
                        };
                        let _ = tracker.add_user_update(user_hash, UserEntryItem {
                            key: key_hash,
                            user_id,
                            user_uuid,
                            uploaded,
                            downloaded,
                            completed,
                            updated,
                            active,
                            torrents_active: BTreeMap::new()
                        }, UpdatesAction::Add);
                    }
                    match tracker.save_user_updates(tracker.clone()).await {
                        Ok(_) => {}
                        Err(_) => {
                            panic!("[IMPORT] Unable to save users to the database!");
                        }
                    }
                }
                Err(error) => {
                    error!("[IMPORT] The users file {} could not be imported!", args.import_file_users.as_str());
                    panic!("[IMPORT] {}", error)
                }
            }
        }

        info!("[IMPORT] Importing of data completed");
        exit(0)
    }
}