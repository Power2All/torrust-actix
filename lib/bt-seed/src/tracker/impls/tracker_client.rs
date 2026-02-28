use crate::config::structs::proxy_config::ProxyConfig;
use crate::tracker::structs::announce_response::AnnounceResponse;
use crate::tracker::structs::http_client::HttpTrackerClient;
use crate::tracker::structs::tracker_client::TrackerClient;
use crate::tracker::structs::udp_client::UdpTrackerClient;

impl TrackerClient {
    pub fn new(
        tracker_url: String,
        info_hash: [u8; 20],
        peer_id: [u8; 20],
        port: u16,
        proxy: Option<&ProxyConfig>,
    ) -> Self {
        if tracker_url.starts_with("udp://") {
            TrackerClient::Udp(UdpTrackerClient::new(tracker_url, info_hash, peer_id, port))
        } else {
            TrackerClient::Http(HttpTrackerClient::new(tracker_url, info_hash, peer_id, port, proxy))
        }
    }

    pub async fn announce(
        &self,
        uploaded: u64,
        event: &str,
    ) -> Result<AnnounceResponse, Box<dyn std::error::Error + Send + Sync>> {
        match self {
            TrackerClient::Http(c) => {
                c.announce(uploaded, event).await.map_err(|e| e.into())
            }
            TrackerClient::Udp(c) => c.announce(uploaded, event).await,
        }
    }
}