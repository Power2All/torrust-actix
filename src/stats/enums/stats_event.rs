use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum StatsEvent {
    Torrents,
    TorrentsUpdates,
    TorrentsShadow,
    Users,
    UsersUpdates,
    UsersShadow,
    TimestampSave,
    TimestampTimeout,
    TimestampConsole,
    TimestampKeysTimeout,
    MaintenanceMode,
    Seeds,
    Peers,
    Completed,
    Whitelist,
    Blacklist,
    Key,
    Tcp4NotFound,
    Tcp4Failure,
    Tcp4ConnectionsHandled,
    Tcp4ApiHandled,
    Tcp4AnnouncesHandled,
    Tcp4ScrapesHandled,
    Tcp6NotFound,
    Tcp6Failure,
    Tcp6ConnectionsHandled,
    Tcp6ApiHandled,
    Tcp6AnnouncesHandled,
    Tcp6ScrapesHandled,
    Udp4ConnectionsHandled,
    Udp4AnnouncesHandled,
    Udp4ScrapesHandled,
    Udp6ConnectionsHandled,
    Udp6AnnouncesHandled,
    Udp6ScrapesHandled,
    TestCounter,
    TestCounterUdp,
}
