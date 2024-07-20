use std::collections::btree_map::Entry;
use std::net::IpAddr;
use std::ops::Deref;
use crate::common::common::get_sys_time_in_secs;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

impl TorrentTracker {
    pub fn validate_throttle(&self, ip: IpAddr) -> bool
    {
        // Parse the IP, so it's the same size.
        let ip_parsed = match ip {
            IpAddr::V4(ip) => { ip.to_ipv6_compatible() }
            IpAddr::V6(ip) => { ip }
        };

        // Check if the IP is in our throttle list, and determine if it hit the throttle limit.
        let map = self.peers_throttler.clone();
        let mut lock = map.write();
        match lock.entry(u128::from_le_bytes(ip_parsed.octets())) {
            Entry::Vacant(_) => {
                true
            }
            Entry::Occupied(mut o) => {
                let (timestamp, count) = o.get_mut();
                if count.deref() <= &self.config.throttle_max_count.unwrap_or(5) {
                    if get_sys_time_in_secs() > timestamp.deref() + self.config.throttle_max_timestamp_reset.unwrap_or(60) {
                        o.remove();
                    }
                    return true;
                }
                if count.deref() > &self.config.throttle_max_count.unwrap_or(5) {
                    if get_sys_time_in_secs() > timestamp.deref() + self.config.throttle_duration_reject.unwrap_or(60) {
                        o.remove();
                        return true;
                    }
                    return false;
                }
                true
            }
        }
    }

    pub fn increase_throttle_count(&self, ip: IpAddr)
    {
        // Parse the IP, so it's the same size.
        let ip_parsed = match ip {
            IpAddr::V4(ip) => { ip.to_ipv6_compatible() }
            IpAddr::V6(ip) => { ip }
        };

        // Check if the IP is in our throttle list, and if not, add it, otherwise update the counter.
        let map = self.peers_throttler.clone();
        let mut lock = map.write();
        match lock.entry(u128::from_le_bytes(ip_parsed.octets())) {
            Entry::Vacant(v) => {
                v.insert((get_sys_time_in_secs(), 1));
            }
            Entry::Occupied(mut o) => {
                let (_, count) = o.get_mut();
                *count += 1;
            }
        }
    }

    pub fn scan_throttle_outdated(&self) -> u64
    {
        let map = self.peers_throttler.clone();
        let lock = map.read();
        let mut remove_list = vec![];
        for (hash, (timestamp, count)) in lock.iter() {
            if count <= &self.config.throttle_max_count.unwrap_or(5) && get_sys_time_in_secs() > timestamp + self.config.throttle_max_timestamp_reset.unwrap_or(60) {
                remove_list.push(*hash);
            }
            if count > &self.config.throttle_max_count.unwrap_or(5) && get_sys_time_in_secs() > timestamp + self.config.throttle_duration_reject.unwrap_or(60) {
                remove_list.push(*hash);
            }
        }
        drop(lock);
        let mut lock = map.write();
        let _: Vec<_> = remove_list.iter().map(|hash| {
            lock.remove(hash);
        }).collect();
        remove_list.len() as u64
    }

    pub fn get_throttle_count(&self) -> u64
    {
        let map = self.peers_throttler.clone();
        let lock = map.read();
        lock.len() as u64
    }
}