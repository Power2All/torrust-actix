use crate::stats::enums::stats_event::StatsEvent;
use crate::stats::structs::stats::Stats;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use std::sync::atomic::{
    AtomicBool,
    AtomicI64,
    Ordering
};

impl TorrentTracker {
    /// Returns a point-in-time copy of all statistics counters.
    pub fn get_stats(&self) -> Stats
    {
        Stats {
            started: self.stats.started.load(Ordering::Relaxed),
            timestamp_run_save: self.stats.timestamp_run_save.load(Ordering::Relaxed),
            timestamp_run_timeout: self.stats.timestamp_run_timeout.load(Ordering::Relaxed),
            timestamp_run_console: self.stats.timestamp_run_console.load(Ordering::Relaxed),
            timestamp_run_keys_timeout: self.stats.timestamp_run_keys_timeout.load(Ordering::Relaxed),
            torrents: self.stats.torrents.load(Ordering::Relaxed),
            torrents_updates: self.stats.torrents_updates.load(Ordering::Relaxed),
            users: self.stats.users.load(Ordering::Relaxed),
            users_updates: self.stats.users_updates.load(Ordering::Relaxed),
            seeds: self.stats.seeds.load(Ordering::Relaxed),
            peers: self.stats.peers.load(Ordering::Relaxed),
            completed: self.stats.completed.load(Ordering::Relaxed),
            whitelist_enabled: self.stats.whitelist_enabled.load(Ordering::Relaxed),
            whitelist: self.stats.whitelist.load(Ordering::Relaxed),
            whitelist_updates: self.stats.whitelist_updates.load(Ordering::Relaxed),
            blacklist_enabled: self.stats.blacklist_enabled.load(Ordering::Relaxed),
            blacklist: self.stats.blacklist.load(Ordering::Relaxed),
            blacklist_updates: self.stats.blacklist_updates.load(Ordering::Relaxed),
            keys_enabled: self.stats.keys_enabled.load(Ordering::Relaxed),
            keys: self.stats.keys.load(Ordering::Relaxed),
            keys_updates: self.stats.keys_updates.load(Ordering::Relaxed),
            tcp4_not_found: self.stats.tcp4_not_found.load(Ordering::Relaxed),
            tcp4_failure: self.stats.tcp4_failure.load(Ordering::Relaxed),
            tcp4_connections_handled: self.stats.tcp4_connections_handled.load(Ordering::Relaxed),
            tcp4_api_handled: self.stats.tcp4_api_handled.load(Ordering::Relaxed),
            tcp4_announces_handled: self.stats.tcp4_announces_handled.load(Ordering::Relaxed),
            tcp4_scrapes_handled: self.stats.tcp4_scrapes_handled.load(Ordering::Relaxed),
            tcp6_not_found: self.stats.tcp6_not_found.load(Ordering::Relaxed),
            tcp6_failure: self.stats.tcp6_failure.load(Ordering::Relaxed),
            tcp6_connections_handled: self.stats.tcp6_connections_handled.load(Ordering::Relaxed),
            tcp6_api_handled: self.stats.tcp6_api_handled.load(Ordering::Relaxed),
            tcp6_announces_handled: self.stats.tcp6_announces_handled.load(Ordering::Relaxed),
            tcp6_scrapes_handled: self.stats.tcp6_scrapes_handled.load(Ordering::Relaxed),
            udp4_bad_request: self.stats.udp4_bad_request.load(Ordering::Relaxed),
            udp4_invalid_request: self.stats.udp4_invalid_request.load(Ordering::Relaxed),
            udp4_connections_handled: self.stats.udp4_connections_handled.load(Ordering::Relaxed),
            udp4_announces_handled: self.stats.udp4_announces_handled.load(Ordering::Relaxed),
            udp4_scrapes_handled: self.stats.udp4_scrapes_handled.load(Ordering::Relaxed),
            udp6_bad_request: self.stats.udp6_bad_request.load(Ordering::Relaxed),
            udp6_invalid_request: self.stats.udp6_invalid_request.load(Ordering::Relaxed),
            udp6_connections_handled: self.stats.udp6_connections_handled.load(Ordering::Relaxed),
            udp6_announces_handled: self.stats.udp6_announces_handled.load(Ordering::Relaxed),
            udp6_scrapes_handled: self.stats.udp6_scrapes_handled.load(Ordering::Relaxed),
            udp_queue_len: self.stats.udp_queue_len.load(Ordering::Relaxed),
            ws_connections_active: self.stats.ws_connections_active.load(Ordering::Relaxed),
            ws_requests_sent: self.stats.ws_requests_sent.load(Ordering::Relaxed),
            ws_requests_received: self.stats.ws_requests_received.load(Ordering::Relaxed),
            ws_responses_sent: self.stats.ws_responses_sent.load(Ordering::Relaxed),
            ws_responses_received: self.stats.ws_responses_received.load(Ordering::Relaxed),
            ws_timeouts: self.stats.ws_timeouts.load(Ordering::Relaxed),
            ws_reconnects: self.stats.ws_reconnects.load(Ordering::Relaxed),
            ws_auth_success: self.stats.ws_auth_success.load(Ordering::Relaxed),
            ws_auth_failed: self.stats.ws_auth_failed.load(Ordering::Relaxed),
        }
    }

    /// Adjusts the IPv4 or the IPv6 variant of a counter, by the family of `ip`.
    #[inline]
    pub fn update_stats_by_family(&self, ip: std::net::IpAddr, ipv4: StatsEvent, ipv6: StatsEvent, value: i64)
    {
        self.update_stats(if ip.is_ipv4() { ipv4 } else { ipv6 }, value);
    }

    /// Adjusts a statistics counter by `value` (negative values decrement).
    ///
    /// Boolean-style events (`WhitelistEnabled`, `BlacklistEnabled`) and gauges (`UdpQueueLen`)
    /// are stored instead of accumulated.
    #[inline]
    pub fn update_stats(&self, event: StatsEvent, value: i64)
    {
        match self.counter(event) {
            Ok(counter) if matches!(event, StatsEvent::UdpQueueLen) => counter.store(value, Ordering::Release),
            Ok(counter) => Self::update_counter(counter, value),
            Err(flag) => flag.store(value > 0, Ordering::Release),
        }
    }

    /// Overwrites a statistics counter with an absolute value.
    pub fn set_stats(&self, event: StatsEvent, value: i64)
    {
        match self.counter(event) {
            Ok(counter) => counter.store(value, Ordering::Release),
            Err(flag) => flag.store(value > 0, Ordering::Release),
        }
    }

    /// The atomic behind `event`: its counter, or `Err` with the flag for the two boolean events.
    #[inline]
    fn counter(&self, event: StatsEvent) -> Result<&AtomicI64, &AtomicBool>
    {
        let s = &self.stats;
        Ok(match event {
            StatsEvent::Torrents => &s.torrents,
            StatsEvent::TorrentsUpdates => &s.torrents_updates,
            StatsEvent::Users => &s.users,
            StatsEvent::UsersUpdates => &s.users_updates,
            StatsEvent::TimestampSave => &s.timestamp_run_save,
            StatsEvent::TimestampTimeout => &s.timestamp_run_timeout,
            StatsEvent::TimestampConsole => &s.timestamp_run_console,
            StatsEvent::TimestampKeysTimeout => &s.timestamp_run_keys_timeout,
            StatsEvent::Seeds => &s.seeds,
            StatsEvent::Peers => &s.peers,
            StatsEvent::Completed => &s.completed,
            StatsEvent::Whitelist => &s.whitelist,
            StatsEvent::WhitelistUpdates => &s.whitelist_updates,
            StatsEvent::Blacklist => &s.blacklist,
            StatsEvent::BlacklistUpdates => &s.blacklist_updates,
            StatsEvent::Key => &s.keys,
            StatsEvent::KeyUpdates => &s.keys_updates,
            StatsEvent::Tcp4NotFound => &s.tcp4_not_found,
            StatsEvent::Tcp4Failure => &s.tcp4_failure,
            StatsEvent::Tcp4ConnectionsHandled => &s.tcp4_connections_handled,
            StatsEvent::Tcp4ApiHandled => &s.tcp4_api_handled,
            StatsEvent::Tcp4AnnouncesHandled => &s.tcp4_announces_handled,
            StatsEvent::Tcp4ScrapesHandled => &s.tcp4_scrapes_handled,
            StatsEvent::Tcp6NotFound => &s.tcp6_not_found,
            StatsEvent::Tcp6Failure => &s.tcp6_failure,
            StatsEvent::Tcp6ConnectionsHandled => &s.tcp6_connections_handled,
            StatsEvent::Tcp6ApiHandled => &s.tcp6_api_handled,
            StatsEvent::Tcp6AnnouncesHandled => &s.tcp6_announces_handled,
            StatsEvent::Tcp6ScrapesHandled => &s.tcp6_scrapes_handled,
            StatsEvent::Udp4BadRequest => &s.udp4_bad_request,
            StatsEvent::Udp4InvalidRequest => &s.udp4_invalid_request,
            StatsEvent::Udp4ConnectionsHandled => &s.udp4_connections_handled,
            StatsEvent::Udp4AnnouncesHandled => &s.udp4_announces_handled,
            StatsEvent::Udp4ScrapesHandled => &s.udp4_scrapes_handled,
            StatsEvent::Udp6BadRequest => &s.udp6_bad_request,
            StatsEvent::Udp6InvalidRequest => &s.udp6_invalid_request,
            StatsEvent::Udp6ConnectionsHandled => &s.udp6_connections_handled,
            StatsEvent::Udp6AnnouncesHandled => &s.udp6_announces_handled,
            StatsEvent::Udp6ScrapesHandled => &s.udp6_scrapes_handled,
            StatsEvent::UdpQueueLen => &s.udp_queue_len,
            StatsEvent::WsConnectionsActive => &s.ws_connections_active,
            StatsEvent::WsRequestsSent => &s.ws_requests_sent,
            StatsEvent::WsRequestsReceived => &s.ws_requests_received,
            StatsEvent::WsResponsesSent => &s.ws_responses_sent,
            StatsEvent::WsResponsesReceived => &s.ws_responses_received,
            StatsEvent::WsTimeouts => &s.ws_timeouts,
            StatsEvent::WsReconnects => &s.ws_reconnects,
            StatsEvent::WsAuthSuccess => &s.ws_auth_success,
            StatsEvent::WsAuthFailed => &s.ws_auth_failed,
            StatsEvent::WhitelistEnabled => return Err(&s.whitelist_enabled),
            StatsEvent::BlacklistEnabled => return Err(&s.blacklist_enabled),
        })
    }

    #[inline(always)]
    fn update_counter(counter: &AtomicI64, value: i64) {
        if value != 0 {
            // Saturating, not `fetch_add`/`fetch_sub`: some deltas carry client-supplied magnitudes
            // (`completed` off the API path, a user's uploaded/downloaded bytes off an announce), so
            // a wrapping counter can be driven negative. `-value` also overflowed on `i64::MIN`.
            // ponytail: a CAS loop rather than `lock xadd`; only matters if a counter ever becomes
            // the announce bottleneck, which it is not next to the shard lock.
            let _ = counter.fetch_update(Ordering::Release, Ordering::Acquire, |current| Some(current.saturating_add(value)));
        }
    }
}