use crate::config::structs::seeder_config::SeederConfig;
use crate::seeder::types::PeerMap;
use crate::torrent::structs::torrent_info::TorrentInfo;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

pub struct Seeder {
    pub config: SeederConfig,
    pub torrent_info: Arc<TorrentInfo>,
    pub peers: PeerMap,
    pub uploaded: Arc<AtomicU64>,
    pub peer_id: [u8; 20],
}