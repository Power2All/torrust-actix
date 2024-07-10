use std::sync::atomic::Ordering;
use crate::stats::enums::stats_event::StatsEvent;
use crate::stats::structs::stats::Stats;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

impl TorrentTracker {
    pub async fn get_stats(&self) -> Stats
    {
        Stats {
            started: self.stats.started.load(Ordering::SeqCst),
            timestamp_run_save: self.stats.timestamp_run_save.load(Ordering::SeqCst),
            timestamp_run_timeout: self.stats.timestamp_run_timeout.load(Ordering::SeqCst),
            timestamp_run_console: self.stats.timestamp_run_console.load(Ordering::SeqCst),
            timestamp_run_keys_timeout: self.stats.timestamp_run_keys_timeout.load(Ordering::SeqCst),
            torrents: self.stats.torrents.load(Ordering::SeqCst),
            torrents_updates: self.stats.torrents_updates.load(Ordering::SeqCst),
            torrents_shadow: self.stats.torrents_shadow.load(Ordering::SeqCst),
            users: self.stats.users.load(Ordering::SeqCst),
            users_updates: self.stats.users_updates.load(Ordering::SeqCst),
            users_shadow: self.stats.users_shadow.load(Ordering::SeqCst),
            maintenance_mode: self.stats.maintenance_mode.load(Ordering::SeqCst),
            seeds: self.stats.seeds.load(Ordering::SeqCst),
            peers: self.stats.peers.load(Ordering::SeqCst),
            completed: self.stats.completed.load(Ordering::SeqCst),
            whitelist_enabled: self.stats.whitelist_enabled.load(Ordering::SeqCst),
            whitelist: self.stats.whitelist.load(Ordering::SeqCst),
            blacklist_enabled: self.stats.blacklist_enabled.load(Ordering::SeqCst),
            blacklist: self.stats.blacklist.load(Ordering::SeqCst),
            keys_enabled: self.stats.keys_enabled.load(Ordering::SeqCst),
            keys: self.stats.keys.load(Ordering::SeqCst),
            tcp4_connections_handled: self.stats.tcp4_connections_handled.load(Ordering::SeqCst),
            tcp4_api_handled: self.stats.tcp4_api_handled.load(Ordering::SeqCst),
            tcp4_announces_handled: self.stats.tcp4_announces_handled.load(Ordering::SeqCst),
            tcp4_scrapes_handled: self.stats.tcp4_scrapes_handled.load(Ordering::SeqCst),
            tcp6_connections_handled: self.stats.tcp6_connections_handled.load(Ordering::SeqCst),
            tcp6_api_handled: self.stats.tcp6_api_handled.load(Ordering::SeqCst),
            tcp6_announces_handled: self.stats.tcp6_announces_handled.load(Ordering::SeqCst),
            tcp6_scrapes_handled: self.stats.tcp6_scrapes_handled.load(Ordering::SeqCst),
            udp4_connections_handled: self.stats.udp4_connections_handled.load(Ordering::SeqCst),
            udp4_announces_handled: self.stats.udp4_announces_handled.load(Ordering::SeqCst),
            udp4_scrapes_handled: self.stats.udp4_scrapes_handled.load(Ordering::SeqCst),
            udp6_connections_handled: self.stats.udp6_connections_handled.load(Ordering::SeqCst),
            udp6_announces_handled: self.stats.udp6_announces_handled.load(Ordering::SeqCst),
            udp6_scrapes_handled: self.stats.udp6_scrapes_handled.load(Ordering::SeqCst),
            test_counter: self.stats.test_counter.load(Ordering::SeqCst),
            test_counter_udp: self.stats.test_counter_udp.load(Ordering::SeqCst),
        }
    }

    pub async fn update_stats(&self, event: StatsEvent, value: i64) -> Stats
    {
        match event {
            StatsEvent::Torrents => {
                if value > 0 { self.stats.torrents.fetch_add(value, Ordering::SeqCst); }
                if value < 0 { self.stats.torrents.fetch_sub(-value, Ordering::SeqCst); }
            }
            StatsEvent::TorrentsUpdates => {
                if value > 0 { self.stats.torrents_updates.fetch_add(value, Ordering::SeqCst); }
                if value < 0 { self.stats.torrents_updates.fetch_sub(-value, Ordering::SeqCst); }
            }
            StatsEvent::TorrentsShadow => {
                if value > 0 { self.stats.torrents_shadow.fetch_add(value, Ordering::SeqCst); }
                if value < 0 { self.stats.torrents_shadow.fetch_sub(-value, Ordering::SeqCst); }
            }
            StatsEvent::Users => {
                if value > 0 { self.stats.users.fetch_add(value, Ordering::SeqCst); }
                if value < 0 { self.stats.users.fetch_sub(-value, Ordering::SeqCst); }
            }
            StatsEvent::UsersUpdates => {
                if value > 0 { self.stats.users_updates.fetch_add(value, Ordering::SeqCst); }
                if value < 0 { self.stats.users_updates.fetch_sub(-value, Ordering::SeqCst); }
            }
            StatsEvent::UsersShadow => {
                if value > 0 { self.stats.users_shadow.fetch_add(value, Ordering::SeqCst); }
                if value < 0 { self.stats.users_shadow.fetch_sub(-value, Ordering::SeqCst); }
            }
            StatsEvent::TimestampSave => {
                if value > 0 { self.stats.timestamp_run_save.fetch_add(value, Ordering::SeqCst); }
                if value < 0 { self.stats.timestamp_run_save.fetch_sub(-value, Ordering::SeqCst); }
            }
            StatsEvent::TimestampTimeout => {
                if value > 0 { self.stats.timestamp_run_timeout.fetch_add(value, Ordering::SeqCst); }
                if value < 0 { self.stats.timestamp_run_timeout.fetch_sub(-value, Ordering::SeqCst); }
            }
            StatsEvent::TimestampConsole => {
                if value > 0 { self.stats.timestamp_run_console.fetch_add(value, Ordering::SeqCst); }
                if value < 0 { self.stats.timestamp_run_console.fetch_sub(-value, Ordering::SeqCst); }
            }
            StatsEvent::TimestampKeysTimeout => {
                if value > 0 { self.stats.timestamp_run_keys_timeout.fetch_add(value, Ordering::SeqCst); }
                if value < 0 { self.stats.timestamp_run_keys_timeout.fetch_sub(-value, Ordering::SeqCst); }
            }
            StatsEvent::MaintenanceMode => {
                if value > 0 { self.stats.maintenance_mode.fetch_add(value, Ordering::SeqCst); }
                if value < 0 { self.stats.maintenance_mode.fetch_sub(-value, Ordering::SeqCst); }
            }
            StatsEvent::Seeds => {
                if value > 0 { self.stats.seeds.fetch_add(value, Ordering::SeqCst); }
                if value < 0 { self.stats.seeds.fetch_sub(-value, Ordering::SeqCst); }
            }
            StatsEvent::Peers => {
                if value > 0 { self.stats.peers.fetch_add(value, Ordering::SeqCst); }
                if value < 0 { self.stats.peers.fetch_sub(-value, Ordering::SeqCst); }
            }
            StatsEvent::Completed => {
                if value > 0 { self.stats.completed.fetch_add(value, Ordering::SeqCst); }
                if value < 0 { self.stats.completed.fetch_sub(-value, Ordering::SeqCst); }
            }
            StatsEvent::Whitelist => {
                if value > 0 { self.stats.whitelist.fetch_add(value, Ordering::SeqCst); }
                if value < 0 { self.stats.whitelist.fetch_sub(-value, Ordering::SeqCst); }
            }
            StatsEvent::Blacklist => {
                if value > 0 { self.stats.blacklist.fetch_add(value, Ordering::SeqCst); }
                if value < 0 { self.stats.blacklist.fetch_sub(-value, Ordering::SeqCst); }
            }
            StatsEvent::Key => {
                if value > 0 { self.stats.keys.fetch_add(value, Ordering::SeqCst); }
                if value < 0 { self.stats.keys.fetch_sub(-value, Ordering::SeqCst); }
            }
            StatsEvent::Tcp4ConnectionsHandled => {
                if value > 0 { self.stats.tcp4_connections_handled.fetch_add(value, Ordering::SeqCst); }
                if value < 0 { self.stats.tcp4_connections_handled.fetch_sub(-value, Ordering::SeqCst); }
            }
            StatsEvent::Tcp4ApiHandled => {
                if value > 0 { self.stats.tcp4_api_handled.fetch_add(value, Ordering::SeqCst); }
                if value < 0 { self.stats.tcp4_api_handled.fetch_sub(-value, Ordering::SeqCst); }
            }
            StatsEvent::Tcp4AnnouncesHandled => {
                if value > 0 { self.stats.tcp4_announces_handled.fetch_add(value, Ordering::SeqCst); }
                if value < 0 { self.stats.tcp4_announces_handled.fetch_sub(-value, Ordering::SeqCst); }
            }
            StatsEvent::Tcp4ScrapesHandled => {
                if value > 0 { self.stats.tcp4_scrapes_handled.fetch_add(value, Ordering::SeqCst); }
                if value < 0 { self.stats.tcp4_scrapes_handled.fetch_sub(-value, Ordering::SeqCst); }
            }
            StatsEvent::Tcp6ConnectionsHandled => {
                if value > 0 { self.stats.tcp6_connections_handled.fetch_add(value, Ordering::SeqCst); }
                if value < 0 { self.stats.tcp6_connections_handled.fetch_sub(-value, Ordering::SeqCst); }
            }
            StatsEvent::Tcp6ApiHandled => {
                if value > 0 { self.stats.tcp6_api_handled.fetch_add(value, Ordering::SeqCst); }
                if value < 0 { self.stats.tcp6_api_handled.fetch_sub(-value, Ordering::SeqCst); }
            }
            StatsEvent::Tcp6AnnouncesHandled => {
                if value > 0 { self.stats.tcp6_announces_handled.fetch_add(value, Ordering::SeqCst); }
                if value < 0 { self.stats.tcp6_announces_handled.fetch_sub(-value, Ordering::SeqCst); }
            }
            StatsEvent::Tcp6ScrapesHandled => {
                if value > 0 { self.stats.tcp6_scrapes_handled.fetch_add(value, Ordering::SeqCst); }
                if value < 0 { self.stats.tcp6_scrapes_handled.fetch_sub(-value, Ordering::SeqCst); }
            }
            StatsEvent::Udp4ConnectionsHandled => {
                if value > 0 { self.stats.udp4_connections_handled.fetch_add(value, Ordering::SeqCst); }
                if value < 0 { self.stats.udp4_connections_handled.fetch_sub(-value, Ordering::SeqCst); }
            }
            StatsEvent::Udp4AnnouncesHandled => {
                if value > 0 { self.stats.udp4_announces_handled.fetch_add(value, Ordering::SeqCst); }
                if value < 0 { self.stats.udp4_announces_handled.fetch_sub(-value, Ordering::SeqCst); }
            }
            StatsEvent::Udp4ScrapesHandled => {
                if value > 0 { self.stats.udp4_scrapes_handled.fetch_add(value, Ordering::SeqCst); }
                if value < 0 { self.stats.udp4_scrapes_handled.fetch_sub(-value, Ordering::SeqCst); }
            }
            StatsEvent::Udp6ConnectionsHandled => {
                if value > 0 { self.stats.udp6_connections_handled.fetch_add(value, Ordering::SeqCst); }
                if value < 0 { self.stats.udp6_connections_handled.fetch_sub(-value, Ordering::SeqCst); }
            }
            StatsEvent::Udp6AnnouncesHandled => {
                if value > 0 { self.stats.udp6_announces_handled.fetch_add(value, Ordering::SeqCst); }
                if value < 0 { self.stats.udp6_announces_handled.fetch_sub(-value, Ordering::SeqCst); }
            }
            StatsEvent::Udp6ScrapesHandled => {
                if value > 0 { self.stats.udp6_scrapes_handled.fetch_add(value, Ordering::SeqCst); }
                if value < 0 { self.stats.udp6_scrapes_handled.fetch_sub(-value, Ordering::SeqCst); }
            }
            StatsEvent::TestCounter => {
                if value > 0 { self.stats.test_counter.fetch_add(value, Ordering::SeqCst); }
                if value < 0 { self.stats.test_counter.fetch_sub(-value, Ordering::SeqCst); }
            }
            StatsEvent::TestCounterUdp => {
                if value > 0 { self.stats.test_counter_udp.fetch_add(value, Ordering::SeqCst); }
                if value < 0 { self.stats.test_counter_udp.fetch_sub(-value, Ordering::SeqCst); }
            }
        }
        self.get_stats().await
    }

    pub async fn set_stats(&self, event: StatsEvent, value: i64) -> Stats
    {
        match event {
            StatsEvent::Torrents => {
                self.stats.torrents.store(value, Ordering::SeqCst);
            }
            StatsEvent::TorrentsUpdates => {
                self.stats.torrents_updates.store(value, Ordering::SeqCst);
            }
            StatsEvent::TorrentsShadow => {
                self.stats.torrents_shadow.store(value, Ordering::SeqCst);
            }
            StatsEvent::Users => {
                self.stats.users.store(value, Ordering::SeqCst);
            }
            StatsEvent::UsersUpdates => {
                self.stats.users_updates.store(value, Ordering::SeqCst);
            }
            StatsEvent::UsersShadow => {
                self.stats.users_shadow.store(value, Ordering::SeqCst);
            }
            StatsEvent::TimestampSave => {
                self.stats.timestamp_run_save.store(value, Ordering::SeqCst);
            }
            StatsEvent::TimestampTimeout => {
                self.stats.timestamp_run_timeout.store(value, Ordering::SeqCst);
            }
            StatsEvent::TimestampConsole => {
                self.stats.timestamp_run_console.store(value, Ordering::SeqCst);
            }
            StatsEvent::TimestampKeysTimeout => {
                self.stats.timestamp_run_keys_timeout.store(value, Ordering::SeqCst);
            }
            StatsEvent::MaintenanceMode => {
                self.stats.maintenance_mode.store(value, Ordering::SeqCst);
            }
            StatsEvent::Seeds => {
                self.stats.seeds.store(value, Ordering::SeqCst);
            }
            StatsEvent::Peers => {
                self.stats.peers.store(value, Ordering::SeqCst);
            }
            StatsEvent::Completed => {
                self.stats.completed.store(value, Ordering::SeqCst);
            }
            StatsEvent::Whitelist => {
                self.stats.whitelist.store(value, Ordering::SeqCst);
            }
            StatsEvent::Blacklist => {
                self.stats.blacklist.store(value, Ordering::SeqCst);
            }
            StatsEvent::Key => {
                self.stats.keys.store(value, Ordering::SeqCst);
            }
            StatsEvent::Tcp4ConnectionsHandled => {
                self.stats.tcp4_connections_handled.store(value, Ordering::SeqCst);
            }
            StatsEvent::Tcp4ApiHandled => {
                self.stats.tcp4_api_handled.store(value, Ordering::SeqCst);
            }
            StatsEvent::Tcp4AnnouncesHandled => {
                self.stats.tcp4_announces_handled.store(value, Ordering::SeqCst);
            }
            StatsEvent::Tcp4ScrapesHandled => {
                self.stats.tcp4_scrapes_handled.store(value, Ordering::SeqCst);
            }
            StatsEvent::Tcp6ConnectionsHandled => {
                self.stats.tcp6_connections_handled.store(value, Ordering::SeqCst);
            }
            StatsEvent::Tcp6ApiHandled => {
                self.stats.tcp6_api_handled.store(value, Ordering::SeqCst);
            }
            StatsEvent::Tcp6AnnouncesHandled => {
                self.stats.tcp6_announces_handled.store(value, Ordering::SeqCst);
            }
            StatsEvent::Tcp6ScrapesHandled => {
                self.stats.tcp6_scrapes_handled.store(value, Ordering::SeqCst);
            }
            StatsEvent::Udp4ConnectionsHandled => {
                self.stats.udp4_connections_handled.store(value, Ordering::SeqCst);
            }
            StatsEvent::Udp4AnnouncesHandled => {
                self.stats.udp4_announces_handled.store(value, Ordering::SeqCst);
            }
            StatsEvent::Udp4ScrapesHandled => {
                self.stats.udp4_scrapes_handled.store(value, Ordering::SeqCst);
            }
            StatsEvent::Udp6ConnectionsHandled => {
                self.stats.udp6_connections_handled.store(value, Ordering::SeqCst);
            }
            StatsEvent::Udp6AnnouncesHandled => {
                self.stats.udp6_announces_handled.store(value, Ordering::SeqCst);
            }
            StatsEvent::Udp6ScrapesHandled => {
                self.stats.udp6_scrapes_handled.store(value, Ordering::SeqCst);
            }
            StatsEvent::TestCounter => {
                self.stats.test_counter.store(value, Ordering::SeqCst);
            }
            StatsEvent::TestCounterUdp => {
                self.stats.test_counter_udp.store(value, Ordering::SeqCst);
            }
        }
        self.get_stats().await
    }
}
