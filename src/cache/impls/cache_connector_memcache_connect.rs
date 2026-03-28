use crate::cache::enums::cache_error::CacheError;
use crate::cache::structs::cache_connector_memcache::CacheConnectorMemcache;
use crate::tracker::structs::info_hash::InfoHash;
use parking_lot::Mutex;
use std::sync::Arc;

impl CacheConnectorMemcache {
    pub fn connect(url: &str, prefix: &str, split_peers: bool) -> Result<Self, CacheError> {
        let client = memcache::connect(url)
            .map_err(|e| CacheError::ConnectionError(format!("Failed to connect to Memcache: {e}")))?;
        Ok(Self {
            client: Arc::new(Mutex::new(client)),
            prefix: prefix.to_string(),
            split_peers,
        })
    }

    pub(crate) fn torrent_key(&self, info_hash: &InfoHash) -> String {
        format!("{}t:{}", self.prefix, info_hash)
    }

    /// Aggregated format: `"seeds:peers:completed"`
    pub(crate) fn serialize_aggregated(seeds: u64, peers: u64, completed: u64) -> String {
        format!("{seeds}:{peers}:{completed}")
    }

    pub(crate) fn deserialize_aggregated(value: &str) -> Option<(u64, u64, u64)> {
        let mut parts = value.splitn(3, ':');
        let seeds = parts.next()?.parse::<u64>().ok()?;
        let peers = parts.next()?.parse::<u64>().ok()?;
        let completed = parts.next().and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
        Some((seeds, peers, completed))
    }

    /// Split format: `"bt_seeds_ipv4:bt_seeds_ipv6:rtc_seeds:bt_peers_ipv4:bt_peers_ipv6:rtc_peers:completed"`
    pub(crate) fn serialize_split(counts: &crate::cache::structs::torrent_peer_counts::TorrentPeerCounts) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}",
            counts.bt_seeds_ipv4, counts.bt_seeds_ipv6, counts.rtc_seeds,
            counts.bt_peers_ipv4, counts.bt_peers_ipv6, counts.rtc_peers,
            counts.completed,
        )
    }

    pub(crate) fn deserialize_split(value: &str) -> Option<crate::cache::structs::torrent_peer_counts::TorrentPeerCounts> {
        let parts: Vec<&str> = value.split(':').collect();
        if parts.len() < 7 {
            return None;
        }
        Some(crate::cache::structs::torrent_peer_counts::TorrentPeerCounts {
            bt_seeds_ipv4: parts[0].parse().ok()?,
            bt_seeds_ipv6: parts[1].parse().ok()?,
            rtc_seeds:     parts[2].parse().ok()?,
            bt_peers_ipv4: parts[3].parse().ok()?,
            bt_peers_ipv6: parts[4].parse().ok()?,
            rtc_peers:     parts[5].parse().ok()?,
            completed:     parts[6].parse().unwrap_or(0),
        })
    }
}