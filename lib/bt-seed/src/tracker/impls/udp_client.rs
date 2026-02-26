use crate::tracker::structs::announce_response::AnnounceResponse;
use crate::tracker::structs::udp_client::UdpTrackerClient;
use crate::tracker::tracker::{
    parse_udp_announce_response,
    parse_udp_connect_response,
    parse_udp_tracker_addr
};
use rand::RngExt;

impl UdpTrackerClient {
    pub fn new(tracker_url: String, info_hash: [u8; 20], peer_id: [u8; 20], port: u16) -> Self {
        Self { tracker_url, info_hash, peer_id, port }
    }

    pub async fn announce(
        &self,
        uploaded: u64,
        event: &str,
    ) -> Result<AnnounceResponse, Box<dyn std::error::Error + Send + Sync>> {
        let addr = parse_udp_tracker_addr(&self.tracker_url)
            .ok_or_else(|| format!("invalid UDP tracker URL: {}", self.tracker_url))?;
        let socket = tokio::net::UdpSocket::bind("0.0.0.0:0").await?;
        let remote_addr = tokio::net::lookup_host(&addr)
            .await?
            .next()
            .ok_or("UDP tracker DNS resolution failed")?;
        socket.connect(remote_addr).await?;
        let txid1: u32 = rand::rng().random();
        let mut connect_req = [0u8; 16];
        connect_req[0..8].copy_from_slice(&0x41727101980u64.to_be_bytes());
        connect_req[8..12].copy_from_slice(&0u32.to_be_bytes());
        connect_req[12..16].copy_from_slice(&txid1.to_be_bytes());
        socket.send(&connect_req).await?;
        let mut resp_buf = [0u8; 16];
        tokio::time::timeout(
            std::time::Duration::from_secs(15),
            socket.recv(&mut resp_buf),
        )
        .await??;
        let connection_id = parse_udp_connect_response(&resp_buf, txid1)
            .ok_or("UDP tracker: invalid connect response")?;
        let txid2: u32 = rand::rng().random();
        let event_num: u32 = match event {
            "started" => 2,
            "stopped" => 3,
            "completed" => 1,
            _ => 0,
        };
        let key: u32 = rand::rng().random();
        let mut ann_req = [0u8; 98];
        ann_req[0..8].copy_from_slice(&connection_id.to_be_bytes());
        ann_req[8..12].copy_from_slice(&1u32.to_be_bytes());
        ann_req[12..16].copy_from_slice(&txid2.to_be_bytes());
        ann_req[16..36].copy_from_slice(&self.info_hash);
        ann_req[36..56].copy_from_slice(&self.peer_id);
        ann_req[56..64].copy_from_slice(&0u64.to_be_bytes());
        ann_req[64..72].copy_from_slice(&0u64.to_be_bytes());
        ann_req[72..80].copy_from_slice(&uploaded.to_be_bytes());
        ann_req[80..84].copy_from_slice(&event_num.to_be_bytes());
        ann_req[84..88].copy_from_slice(&0u32.to_be_bytes());
        ann_req[88..92].copy_from_slice(&key.to_be_bytes());
        ann_req[92..96].copy_from_slice(&(-1i32).to_be_bytes());
        ann_req[96..98].copy_from_slice(&self.port.to_be_bytes());
        socket.send(&ann_req).await?;
        let mut ann_resp = [0u8; 1024];
        let n = tokio::time::timeout(
            std::time::Duration::from_secs(15),
            socket.recv(&mut ann_resp),
        )
        .await??;
        let resp = parse_udp_announce_response(&ann_resp[..n], txid2)
            .ok_or("UDP tracker: invalid announce response")?;
        log::debug!("[Tracker/UDP] Announce OK: interval={}s", resp.interval);
        Ok(resp)
    }
}