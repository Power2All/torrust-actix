use crate::stats::enums::stats_event::StatsEvent;
use crate::stats::structs::stats::Stats;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use std::sync::atomic::Ordering;

impl TorrentTracker {
    #[tracing::instrument(level = "debug")]
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
            wt4_connections_handled: self.stats.wt4_connections_handled.load(Ordering::Relaxed),
            wt4_announces_handled: self.stats.wt4_announces_handled.load(Ordering::Relaxed),
            wt4_offers_handled: self.stats.wt4_offers_handled.load(Ordering::Relaxed),
            wt4_answers_handled: self.stats.wt4_answers_handled.load(Ordering::Relaxed),
            wt4_scrapes_handled: self.stats.wt4_scrapes_handled.load(Ordering::Relaxed),
            wt4_failure: self.stats.wt4_failure.load(Ordering::Relaxed),
            wt6_connections_handled: self.stats.wt6_connections_handled.load(Ordering::Relaxed),
            wt6_announces_handled: self.stats.wt6_announces_handled.load(Ordering::Relaxed),
            wt6_offers_handled: self.stats.wt6_offers_handled.load(Ordering::Relaxed),
            wt6_answers_handled: self.stats.wt6_answers_handled.load(Ordering::Relaxed),
            wt6_scrapes_handled: self.stats.wt6_scrapes_handled.load(Ordering::Relaxed),
            wt6_failure: self.stats.wt6_failure.load(Ordering::Relaxed),
        }
    }

    #[tracing::instrument(level = "debug")]
    #[inline]
    pub fn update_stats(&self, event: StatsEvent, value: i64) -> Stats
    {
        match event {
            StatsEvent::Torrents => {
                self.update_counter(&self.stats.torrents, value);
            }
            StatsEvent::TorrentsUpdates => {
                self.update_counter(&self.stats.torrents_updates, value);
            }
            StatsEvent::Users => {
                self.update_counter(&self.stats.users, value);
            }
            StatsEvent::UsersUpdates => {
                self.update_counter(&self.stats.users_updates, value);
            }
            StatsEvent::TimestampSave => {
                self.update_counter(&self.stats.timestamp_run_save, value);
            }
            StatsEvent::TimestampTimeout => {
                self.update_counter(&self.stats.timestamp_run_timeout, value);
            }
            StatsEvent::TimestampConsole => {
                self.update_counter(&self.stats.timestamp_run_console, value);
            }
            StatsEvent::TimestampKeysTimeout => {
                self.update_counter(&self.stats.timestamp_run_keys_timeout, value);
            }
            StatsEvent::Seeds => {
                self.update_counter(&self.stats.seeds, value);
            }
            StatsEvent::Peers => {
                self.update_counter(&self.stats.peers, value);
            }
            StatsEvent::Completed => {
                self.update_counter(&self.stats.completed, value);
            }
            StatsEvent::WhitelistEnabled => {
                self.stats.whitelist_enabled.store(value > 0, Ordering::Release);
            }
            StatsEvent::Whitelist => {
                self.update_counter(&self.stats.whitelist, value);
            }
            StatsEvent::WhitelistUpdates => {
                self.update_counter(&self.stats.whitelist_updates, value);
            }
            StatsEvent::BlacklistEnabled => {
                self.stats.blacklist_enabled.store(value > 0, Ordering::Release);
            }
            StatsEvent::Blacklist => {
                self.update_counter(&self.stats.blacklist, value);
            }
            StatsEvent::BlacklistUpdates => {
                self.update_counter(&self.stats.blacklist_updates, value);
            }
            StatsEvent::Key => {
                self.update_counter(&self.stats.keys, value);
            }
            StatsEvent::KeyUpdates => {
                self.update_counter(&self.stats.keys_updates, value);
            }
            StatsEvent::Tcp4NotFound => {
                self.update_counter(&self.stats.tcp4_not_found, value);
            }
            StatsEvent::Tcp4Failure => {
                self.update_counter(&self.stats.tcp4_failure, value);
            }
            StatsEvent::Tcp4ConnectionsHandled => {
                self.update_counter(&self.stats.tcp4_connections_handled, value);
            }
            StatsEvent::Tcp4ApiHandled => {
                self.update_counter(&self.stats.tcp4_api_handled, value);
            }
            StatsEvent::Tcp4AnnouncesHandled => {
                self.update_counter(&self.stats.tcp4_announces_handled, value);
            }
            StatsEvent::Tcp4ScrapesHandled => {
                self.update_counter(&self.stats.tcp4_scrapes_handled, value);
            }
            StatsEvent::Tcp6NotFound => {
                self.update_counter(&self.stats.tcp6_not_found, value);
            }
            StatsEvent::Tcp6Failure => {
                self.update_counter(&self.stats.tcp6_failure, value);
            }
            StatsEvent::Tcp6ConnectionsHandled => {
                self.update_counter(&self.stats.tcp6_connections_handled, value);
            }
            StatsEvent::Tcp6ApiHandled => {
                self.update_counter(&self.stats.tcp6_api_handled, value);
            }
            StatsEvent::Tcp6AnnouncesHandled => {
                self.update_counter(&self.stats.tcp6_announces_handled, value);
            }
            StatsEvent::Tcp6ScrapesHandled => {
                self.update_counter(&self.stats.tcp6_scrapes_handled, value);
            }
            StatsEvent::Udp4BadRequest => {
                self.update_counter(&self.stats.udp4_bad_request, value);
            }
            StatsEvent::Udp4InvalidRequest => {
                self.update_counter(&self.stats.udp4_invalid_request, value);
            }
            StatsEvent::Udp4ConnectionsHandled => {
                self.update_counter(&self.stats.udp4_connections_handled, value);
            }
            StatsEvent::Udp4AnnouncesHandled => {
                self.update_counter(&self.stats.udp4_announces_handled, value);
            }
            StatsEvent::Udp4ScrapesHandled => {
                self.update_counter(&self.stats.udp4_scrapes_handled, value);
            }
            StatsEvent::Udp6BadRequest => {
                self.update_counter(&self.stats.udp6_bad_request, value);
            }
            StatsEvent::Udp6InvalidRequest => {
                self.update_counter(&self.stats.udp6_invalid_request, value);
            }
            StatsEvent::Udp6ConnectionsHandled => {
                self.update_counter(&self.stats.udp6_connections_handled, value);
            }
            StatsEvent::Udp6AnnouncesHandled => {
                self.update_counter(&self.stats.udp6_announces_handled, value);
            }
            StatsEvent::Udp6ScrapesHandled => {
                self.update_counter(&self.stats.udp6_scrapes_handled, value);
            }
            StatsEvent::UdpQueueLen => {
                self.stats.udp_queue_len.store(value, Ordering::Release);
            }
            
            StatsEvent::WsConnectionsActive => {
                self.update_counter(&self.stats.ws_connections_active, value);
            }
            StatsEvent::WsRequestsSent => {
                self.update_counter(&self.stats.ws_requests_sent, value);
            }
            StatsEvent::WsRequestsReceived => {
                self.update_counter(&self.stats.ws_requests_received, value);
            }
            StatsEvent::WsResponsesSent => {
                self.update_counter(&self.stats.ws_responses_sent, value);
            }
            StatsEvent::WsResponsesReceived => {
                self.update_counter(&self.stats.ws_responses_received, value);
            }
            StatsEvent::WsTimeouts => {
                self.update_counter(&self.stats.ws_timeouts, value);
            }
            StatsEvent::WsReconnects => {
                self.update_counter(&self.stats.ws_reconnects, value);
            }
            StatsEvent::WsAuthSuccess => {
                self.update_counter(&self.stats.ws_auth_success, value);
            }
            StatsEvent::WsAuthFailed => {
                self.update_counter(&self.stats.ws_auth_failed, value);
            }
            StatsEvent::Wt4ConnectionsHandled => {
                self.update_counter(&self.stats.wt4_connections_handled, value);
            }
            StatsEvent::Wt4AnnouncesHandled => {
                self.update_counter(&self.stats.wt4_announces_handled, value);
            }
            StatsEvent::Wt4OffersHandled => {
                self.update_counter(&self.stats.wt4_offers_handled, value);
            }
            StatsEvent::Wt4AnswersHandled => {
                self.update_counter(&self.stats.wt4_answers_handled, value);
            }
            StatsEvent::Wt4ScrapesHandled => {
                self.update_counter(&self.stats.wt4_scrapes_handled, value);
            }
            StatsEvent::Wt4Failure => {
                self.update_counter(&self.stats.wt4_failure, value);
            }
            StatsEvent::Wt6ConnectionsHandled => {
                self.update_counter(&self.stats.wt6_connections_handled, value);
            }
            StatsEvent::Wt6AnnouncesHandled => {
                self.update_counter(&self.stats.wt6_announces_handled, value);
            }
            StatsEvent::Wt6OffersHandled => {
                self.update_counter(&self.stats.wt6_offers_handled, value);
            }
            StatsEvent::Wt6AnswersHandled => {
                self.update_counter(&self.stats.wt6_answers_handled, value);
            }
            StatsEvent::Wt6ScrapesHandled => {
                self.update_counter(&self.stats.wt6_scrapes_handled, value);
            }
            StatsEvent::Wt6Failure => {
                self.update_counter(&self.stats.wt6_failure, value);
            }
        };
        self.get_stats()
    }

    #[tracing::instrument(level = "debug")]
    pub fn set_stats(&self, event: StatsEvent, value: i64) -> Stats
    {
        match event {
            StatsEvent::Torrents => {
                self.stats.torrents.store(value, Ordering::Release);
            }
            StatsEvent::TorrentsUpdates => {
                self.stats.torrents_updates.store(value, Ordering::Release);
            }
            StatsEvent::Users => {
                self.stats.users.store(value, Ordering::Release);
            }
            StatsEvent::UsersUpdates => {
                self.stats.users_updates.store(value, Ordering::Release);
            }
            StatsEvent::TimestampSave => {
                self.stats.timestamp_run_save.store(value, Ordering::Release);
            }
            StatsEvent::TimestampTimeout => {
                self.stats.timestamp_run_timeout.store(value, Ordering::Release);
            }
            StatsEvent::TimestampConsole => {
                self.stats.timestamp_run_console.store(value, Ordering::Release);
            }
            StatsEvent::TimestampKeysTimeout => {
                self.stats.timestamp_run_keys_timeout.store(value, Ordering::Release);
            }
            StatsEvent::Seeds => {
                self.stats.seeds.store(value, Ordering::Release);
            }
            StatsEvent::Peers => {
                self.stats.peers.store(value, Ordering::Release);
            }
            StatsEvent::Completed => {
                self.stats.completed.store(value, Ordering::Release);
            }
            StatsEvent::WhitelistEnabled => {
                self.stats.whitelist_enabled.store(value > 0, Ordering::Release);
            }
            StatsEvent::Whitelist => {
                self.stats.whitelist.store(value, Ordering::Release);
            }
            StatsEvent::WhitelistUpdates => {
                self.stats.whitelist_updates.store(value, Ordering::Release);
            }
            StatsEvent::BlacklistEnabled => {
                self.stats.blacklist_enabled.store(value > 0, Ordering::Release);
            }
            StatsEvent::Blacklist => {
                self.stats.blacklist.store(value, Ordering::Release);
            }
            StatsEvent::BlacklistUpdates => {
                self.stats.blacklist_updates.store(value, Ordering::Release);
            }
            StatsEvent::Key => {
                self.stats.keys.store(value, Ordering::Release);
            }
            StatsEvent::KeyUpdates => {
                self.stats.keys_updates.store(value, Ordering::Release);
            }
            StatsEvent::Tcp4NotFound => {
                self.stats.tcp4_not_found.store(value, Ordering::Release);
            }
            StatsEvent::Tcp4Failure => {
                self.stats.tcp4_failure.store(value, Ordering::Release);
            }
            StatsEvent::Tcp4ConnectionsHandled => {
                self.stats.tcp4_connections_handled.store(value, Ordering::Release);
            }
            StatsEvent::Tcp4ApiHandled => {
                self.stats.tcp4_api_handled.store(value, Ordering::Release);
            }
            StatsEvent::Tcp4AnnouncesHandled => {
                self.stats.tcp4_announces_handled.store(value, Ordering::Release);
            }
            StatsEvent::Tcp4ScrapesHandled => {
                self.stats.tcp4_scrapes_handled.store(value, Ordering::Release);
            }
            StatsEvent::Tcp6NotFound => {
                self.stats.tcp6_not_found.store(value, Ordering::Release);
            }
            StatsEvent::Tcp6Failure => {
                self.stats.tcp6_failure.store(value, Ordering::Release);
            }
            StatsEvent::Tcp6ConnectionsHandled => {
                self.stats.tcp6_connections_handled.store(value, Ordering::Release);
            }
            StatsEvent::Tcp6ApiHandled => {
                self.stats.tcp6_api_handled.store(value, Ordering::Release);
            }
            StatsEvent::Tcp6AnnouncesHandled => {
                self.stats.tcp6_announces_handled.store(value, Ordering::Release);
            }
            StatsEvent::Tcp6ScrapesHandled => {
                self.stats.tcp6_scrapes_handled.store(value, Ordering::Release);
            }
            StatsEvent::Udp4BadRequest => {
                self.stats.udp4_bad_request.store(value, Ordering::Release);
            }
            StatsEvent::Udp4InvalidRequest => {
                self.stats.udp4_invalid_request.store(value, Ordering::Release);
            }
            StatsEvent::Udp4ConnectionsHandled => {
                self.stats.udp4_connections_handled.store(value, Ordering::Release);
            }
            StatsEvent::Udp4AnnouncesHandled => {
                self.stats.udp4_announces_handled.store(value, Ordering::Release);
            }
            StatsEvent::Udp4ScrapesHandled => {
                self.stats.udp4_scrapes_handled.store(value, Ordering::Release);
            }
            StatsEvent::Udp6BadRequest => {
                self.stats.udp6_bad_request.store(value, Ordering::Release);
            }
            StatsEvent::Udp6InvalidRequest => {
                self.stats.udp6_invalid_request.store(value, Ordering::Release);
            }
            StatsEvent::Udp6ConnectionsHandled => {
                self.stats.udp6_connections_handled.store(value, Ordering::Release);
            }
            StatsEvent::Udp6AnnouncesHandled => {
                self.stats.udp6_announces_handled.store(value, Ordering::Release);
            }
            StatsEvent::Udp6ScrapesHandled => {
                self.stats.udp6_scrapes_handled.store(value, Ordering::Release);
            }
            StatsEvent::UdpQueueLen => {
                self.stats.udp_queue_len.store(value, Ordering::Release);
            }
            StatsEvent::WsConnectionsActive => {
                self.stats.ws_connections_active.store(value, Ordering::Release);
            }
            StatsEvent::WsRequestsSent => {
                self.stats.ws_requests_sent.store(value, Ordering::Release);
            }
            StatsEvent::WsRequestsReceived => {
                self.stats.ws_requests_received.store(value, Ordering::Release);
            }
            StatsEvent::WsResponsesSent => {
                self.stats.ws_responses_sent.store(value, Ordering::Release);
            }
            StatsEvent::WsResponsesReceived => {
                self.stats.ws_responses_received.store(value, Ordering::Release);
            }
            StatsEvent::WsTimeouts => {
                self.stats.ws_timeouts.store(value, Ordering::Release);
            }
            StatsEvent::WsReconnects => {
                self.stats.ws_reconnects.store(value, Ordering::Release);
            }
            StatsEvent::WsAuthSuccess => {
                self.stats.ws_auth_success.store(value, Ordering::Release);
            }
            StatsEvent::WsAuthFailed => {
                self.stats.ws_auth_failed.store(value, Ordering::Release);
            }
            StatsEvent::Wt4ConnectionsHandled => {
                self.stats.wt4_connections_handled.store(value, Ordering::Release);
            }
            StatsEvent::Wt4AnnouncesHandled => {
                self.stats.wt4_announces_handled.store(value, Ordering::Release);
            }
            StatsEvent::Wt4OffersHandled => {
                self.stats.wt4_offers_handled.store(value, Ordering::Release);
            }
            StatsEvent::Wt4AnswersHandled => {
                self.stats.wt4_answers_handled.store(value, Ordering::Release);
            }
            StatsEvent::Wt4ScrapesHandled => {
                self.stats.wt4_scrapes_handled.store(value, Ordering::Release);
            }
            StatsEvent::Wt4Failure => {
                self.stats.wt4_failure.store(value, Ordering::Release);
            }
            StatsEvent::Wt6ConnectionsHandled => {
                self.stats.wt6_connections_handled.store(value, Ordering::Release);
            }
            StatsEvent::Wt6AnnouncesHandled => {
                self.stats.wt6_announces_handled.store(value, Ordering::Release);
            }
            StatsEvent::Wt6OffersHandled => {
                self.stats.wt6_offers_handled.store(value, Ordering::Release);
            }
            StatsEvent::Wt6AnswersHandled => {
                self.stats.wt6_answers_handled.store(value, Ordering::Release);
            }
            StatsEvent::Wt6ScrapesHandled => {
                self.stats.wt6_scrapes_handled.store(value, Ordering::Release);
            }
            StatsEvent::Wt6Failure => {
                self.stats.wt6_failure.store(value, Ordering::Release);
            }
        };
        self.get_stats()
    }

    #[inline(always)]
    fn update_counter(&self, counter: &std::sync::atomic::AtomicI64, value: i64) {
        if value > 0 {
            counter.fetch_add(value, Ordering::Release);
        } else if value < 0 {
            counter.fetch_sub(-value, Ordering::Release);
        }
    }
}