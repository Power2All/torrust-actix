mod common;

use actix_web::{
    test,
    App
};
use std::collections::{
    BTreeMap,
    HashMap
};
use std::sync::Arc;
use torrust_actix::cache::enums::cache_engine::CacheEngine;
use torrust_actix::config::structs::cache_config::CacheConfig;
use torrust_actix::config::structs::configuration::Configuration;
use torrust_actix::http::http::http_service_routes;
use torrust_actix::http::structs::http_service_data::HttpServiceData;
use torrust_actix::tracker::enums::updates_action::UpdatesAction;
use torrust_actix::tracker::structs::torrent_tracker::TorrentTracker;
use torrust_actix::tracker::structs::user_entry_item::UserEntryItem;
use torrust_actix::tracker::structs::user_id::UserId;

const USER_ID: UserId = UserId([1; 20]);
const USER_KEY: UserId = UserId([0x12; 20]);

fn cache_config(engine: CacheEngine, address: &str, prefix: &str) -> CacheConfig {
    CacheConfig {
        enabled: true,
        engine,
        address: address.to_string(),
        prefix: prefix.to_string(),
        ttl: 300,
        split_peers: false,
    }
}

/// Users on, nothing persistent: the cache is the only thing that reads user changes.
async fn tracker_with(cache: Option<CacheConfig>) -> Arc<TorrentTracker> {
    let config: Configuration = common::build_test_config(|c| {
        c.tracker_config.users_enabled = true;
        c.cache = cache;
    });
    let tracker = Arc::new(TorrentTracker::new(Arc::new(config), false).await);
    tracker.add_user(USER_ID, user(0, 0));
    tracker
}

fn user(uploaded: u64, downloaded: u64) -> UserEntryItem {
    UserEntryItem {
        key: USER_KEY,
        user_id: Some(7),
        user_uuid: None,
        uploaded,
        downloaded,
        completed: 0,
        updated: 1_700_000_000,
        active: 1,
        torrents_active: BTreeMap::new(),
    }
}

#[actix_web::test]
async fn test_announce_queues_user_changes_for_the_cache() {
    // Announces used to queue a user change only for a persistent users table, so with the cache
    // as the only reader the change was never written anywhere.
    // (cache on, queued)
    for (cache, queued) in [(true, true), (false, false)] {
        // Nothing listens on port 1: the flush must still run and retire the queue, only logging
        // the failed cache write.
        let tracker = tracker_with(cache.then(|| cache_config(CacheEngine::redis, "127.0.0.1:1", ""))).await;
        let app = test::init_service(App::new().configure(http_service_routes(Arc::new(HttpServiceData {
            torrent_tracker: tracker.clone(),
            http_trackers_config: common::create_test_http_config(),
        })))).await;
        let req = test::TestRequest::get()
            .uri(&format!(
                "/{}/announce?info_hash={}&peer_id={}&port=6881&uploaded=0&downloaded=0&left=1000&compact=1&event=started",
                hex::encode(USER_KEY.0),
                common::percent_encode(&common::random_info_hash().0),
                common::percent_encode(&common::random_peer_id().0),
            ))
            .peer_addr("127.0.0.1:0".parse().unwrap())
            .to_request();
        test::call_service(&app, req).await;
        assert_eq!(tracker.get_user_updates().contains_key(&USER_ID), queued, "cache = {cache}");
        TorrentTracker::save_all_updates(&tracker).await;
        assert!(tracker.get_user_updates().is_empty(), "cache = {cache}: the flush runs without a persistent table");
    }
}

/// A prefix of its own, so a run never reads or deletes keys it did not write.
fn test_prefix() -> String {
    format!("test{}:", hex::encode(common::random_peer_id().0))
}

#[tokio::test]
#[ignore = "needs a Redis server: TEST_REDIS_ADDRESS=127.0.0.1:6379 cargo test --test cache_tests -- --ignored"]
async fn test_users_in_redis() {
    let address = std::env::var("TEST_REDIS_ADDRESS").unwrap_or_else(|_| "127.0.0.1:6379".to_string());
    let prefix = test_prefix();
    let tracker = tracker_with(Some(cache_config(CacheEngine::redis, &address, &prefix))).await;
    let key = format!("{prefix}u:7");
    let mut conn = redis::Client::open(format!("redis://{address}")).unwrap().get_multiplexed_async_connection().await.unwrap();

    tracker.add_user_update(USER_ID, user(5 << 30, 300), UpdatesAction::Add);
    tracker.save_user_updates(tracker.clone()).await.unwrap();
    let fields: HashMap<String, u64> = redis::AsyncCommands::hgetall(&mut conn, &key).await.unwrap();
    assert_eq!(fields, HashMap::from([
        ("uploaded".to_string(), 5 << 30),
        ("downloaded".to_string(), 300),
        ("completed".to_string(), 0),
        ("active".to_string(), 1),
        ("updated".to_string(), 1_700_000_000),
        ("torrents".to_string(), 0),
    ]));
    let ttl: i64 = redis::AsyncCommands::ttl(&mut conn, &key).await.unwrap();
    assert_eq!(ttl, -1, "user keys do not take the torrents' TTL");

    tracker.add_user_update(USER_ID, user(5 << 30, 300), UpdatesAction::Remove);
    tracker.save_user_updates(tracker.clone()).await.unwrap();
    let exists: bool = redis::AsyncCommands::exists(&mut conn, &key).await.unwrap();
    assert!(!exists, "a removed user is deleted from the cache");
}

#[tokio::test]
#[ignore = "needs a Memcache server: TEST_MEMCACHE_ADDRESS=127.0.0.1:11211 cargo test --test cache_tests -- --ignored"]
async fn test_users_in_memcache() {
    let address = std::env::var("TEST_MEMCACHE_ADDRESS").unwrap_or_else(|_| "127.0.0.1:11211".to_string());
    let prefix = test_prefix();
    let tracker = tracker_with(Some(cache_config(CacheEngine::memcache, &address, &prefix))).await;
    let key = format!("{prefix}u:7");
    let client = memcache::connect(format!("memcache://{address}")).unwrap();

    tracker.add_user_update(USER_ID, user(5 << 30, 300), UpdatesAction::Add);
    tracker.save_user_updates(tracker.clone()).await.unwrap();
    let value: Option<String> = client.get(&key).unwrap();
    assert_eq!(value.as_deref(), Some("5368709120:300:0:1:1700000000:0"));

    tracker.add_user_update(USER_ID, user(5 << 30, 300), UpdatesAction::Remove);
    tracker.save_user_updates(tracker.clone()).await.unwrap();
    let value: Option<String> = client.get(&key).unwrap();
    assert_eq!(value, None, "a removed user is deleted from the cache");
}
