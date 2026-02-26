use crate::seeder::structs::peer_count_guard::PeerCountGuard;
use std::sync::atomic::Ordering;

impl Drop for PeerCountGuard {
    fn drop(&mut self) {
        self.count.fetch_sub(1, Ordering::Relaxed);
    }
}