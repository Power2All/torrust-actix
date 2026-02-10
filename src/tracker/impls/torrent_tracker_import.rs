use crate::structs::Cli;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::tracker::structs::user_entry_item::UserEntryItem;
use crate::tracker::structs::user_id::UserId;
use log::{
    error,
    info
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::process::exit;
use std::sync::Arc;

impl TorrentTracker {
    #[tracing::instrument(level = "debug")]
    pub async fn import(&self, args: &Cli, tracker: Arc<TorrentTracker>)
    {
        info!("[IMPORT] Requesting to import data");
        let config = &tracker.config.tracker_config;
        let torrents_file = &args.import_file_torrents;
        info!("[IMPORT] Importing torrents to memory {torrents_file}");
        let data = fs::read(torrents_file)
            .unwrap_or_else(|error| {
                error!("[IMPORT] The torrents file {torrents_file} could not be imported!");
                panic!("[IMPORT] {error}")
            });
        let torrents: Value = serde_json::from_slice(&data)
            .expect("[IMPORT] Failed to parse torrents JSON");
        let torrents_obj = torrents.as_object()
            .expect("[IMPORT] Torrents data is not a JSON object");
        for (key, value) in torrents_obj {
            let completed = value["completed"].as_u64()
                .expect("[IMPORT] 'completed' field doesn't exist or is missing!");
            let hash_bytes = hex::decode(key)
                .expect("[IMPORT] Torrent hash is not hex or invalid!");
            let info_hash = InfoHash(hash_bytes[..20].try_into()
                .expect("[IMPORT] Invalid hash length"));
            tracker.add_torrent_update(info_hash, TorrentEntry {
                seeds: Default::default(),
                peers: Default::default(),
                completed,
                updated: std::time::Instant::now(),
            }, UpdatesAction::Add);
        }
        tracker.save_torrent_updates(Arc::clone(&tracker)).await
            .expect("[IMPORT] Unable to save torrents to the database!");
        if config.whitelist_enabled {
            let whitelists_file = &args.import_file_whitelists;
            info!("[IMPORT] Importing whitelists to memory {whitelists_file}");
            let data = fs::read(whitelists_file)
                .unwrap_or_else(|error| {
                    error!("[IMPORT] The whitelists file {whitelists_file} could not be imported!");
                    panic!("[IMPORT] {error}")
                });
            let whitelists: Value = serde_json::from_slice(&data)
                .expect("[IMPORT] Failed to parse whitelists JSON");
            let whitelists_array = whitelists.as_array()
                .expect("[IMPORT] Whitelists data is not a JSON array");
            for value in whitelists_array {
                let hash_str = value.as_str()
                    .expect("[IMPORT] Whitelist entry is not a string");
                let hash_bytes = hex::decode(hash_str)
                    .expect("[IMPORT] Torrent hash is not hex or invalid!");
                let info_hash = InfoHash(hash_bytes[..20].try_into()
                    .expect("[IMPORT] Invalid hash length"));
                tracker.add_whitelist_update(info_hash, UpdatesAction::Add);
            }
            tracker.save_whitelist_updates(Arc::clone(&tracker)).await
                .expect("[IMPORT] Unable to save whitelist to the database!");
        }
        if config.blacklist_enabled {
            let blacklists_file = &args.import_file_blacklists;
            info!("[IMPORT] Importing blacklists to memory {blacklists_file}");
            let data = fs::read(blacklists_file)
                .unwrap_or_else(|error| {
                    error!("[IMPORT] The blacklists file {blacklists_file} could not be imported!");
                    panic!("[IMPORT] {error}")
                });
            let blacklists: Value = serde_json::from_slice(&data)
                .expect("[IMPORT] Failed to parse blacklists JSON");
            let blacklists_array = blacklists.as_array()
                .expect("[IMPORT] Blacklists data is not a JSON array");
            for value in blacklists_array {
                let hash_str = value.as_str()
                    .expect("[IMPORT] Blacklist entry is not a string");
                let hash_bytes = hex::decode(hash_str)
                    .expect("[IMPORT] Torrent hash is not hex or invalid!");
                let info_hash = InfoHash(hash_bytes[..20].try_into()
                    .expect("[IMPORT] Invalid hash length"));
                tracker.add_blacklist_update(info_hash, UpdatesAction::Add);
            }
            tracker.save_blacklist_updates(Arc::clone(&tracker)).await
                .expect("[IMPORT] Unable to save blacklist to the database!");
        }
        if config.keys_enabled {
            let keys_file = &args.import_file_keys;
            info!("[IMPORT] Importing keys to memory {keys_file}");
            let data = fs::read(keys_file)
                .unwrap_or_else(|error| {
                    error!("[IMPORT] The keys file {keys_file} could not be imported!");
                    panic!("[IMPORT] {error}")
                });
            let keys: Value = serde_json::from_slice(&data)
                .expect("[IMPORT] Failed to parse keys JSON");
            let keys_obj = keys.as_object()
                .expect("[IMPORT] Keys data is not a JSON object");
            for (key, value) in keys_obj {
                let timeout = value.as_i64()
                    .expect("[IMPORT] timeout value doesn't exist or is missing!");
                let hash_bytes = hex::decode(key)
                    .expect("[IMPORT] Key hash is not hex or invalid!");
                let hash = InfoHash(hash_bytes[..20].try_into()
                    .expect("[IMPORT] Invalid hash length"));
                tracker.add_key_update(hash, timeout, UpdatesAction::Add);
            }
            tracker.save_key_updates(Arc::clone(&tracker)).await
                .expect("[IMPORT] Unable to save keys to the database!");
        }
        if config.users_enabled {
            let users_file = &args.import_file_users;
            info!("[IMPORT] Importing users to memory {users_file}");
            let data = fs::read(users_file)
                .unwrap_or_else(|error| {
                    error!("[IMPORT] The users file {users_file} could not be imported!");
                    panic!("[IMPORT] {error}")
                });
            let users: Value = serde_json::from_slice(&data)
                .expect("[IMPORT] Failed to parse users JSON");
            let users_obj = users.as_object()
                .expect("[IMPORT] Users data is not a JSON object");
            for (key, value) in users_obj {
                let user_hash_bytes = hex::decode(key)
                    .expect("[IMPORT] User hash is not hex or invalid!");
                let user_hash = UserId(user_hash_bytes[..20].try_into()
                    .expect("[IMPORT] Invalid hash length"));
                let key_str = value["key"].as_str()
                    .expect("[IMPORT] Key field is missing or not a string");
                let key_hash_bytes = hex::decode(key_str)
                    .expect("[IMPORT] Key hash is not hex or invalid!");
                let key_hash = UserId(key_hash_bytes[..20].try_into()
                    .expect("[IMPORT] Invalid hash length"));
                let user_entry = UserEntryItem {
                    key: key_hash,
                    user_id: value["user_id"].as_u64(),
                    user_uuid: value["user_uuid"].as_str().map(String::from),
                    uploaded: value["uploaded"].as_u64()
                        .expect("[IMPORT] 'uploaded' field doesn't exist or is missing!"),
                    downloaded: value["downloaded"].as_u64()
                        .expect("[IMPORT] 'downloaded' field doesn't exist or is missing!"),
                    completed: value["completed"].as_u64()
                        .expect("[IMPORT] 'completed' field doesn't exist or is missing!"),
                    updated: value["updated"].as_u64()
                        .expect("[IMPORT] 'updated' field doesn't exist or is missing!"),
                    active: value["active"].as_u64()
                        .expect("[IMPORT] 'active' field doesn't exist or is missing!") as u8,
                    torrents_active: BTreeMap::new()
                };
                tracker.add_user_update(user_hash, user_entry, UpdatesAction::Add);
            }
            tracker.save_user_updates(Arc::clone(&tracker)).await
                .expect("[IMPORT] Unable to save users to the database!");
        }
        info!("[IMPORT] Importing of data completed");
        exit(0)
    }
}