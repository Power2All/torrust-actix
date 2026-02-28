use crate::config::structs::proxy_config::ProxyConfig;
use crate::tracker::structs::announce_response::AnnounceResponse;
use crate::tracker::structs::http_client::HttpTrackerClient;
use crate::tracker::tracker::{build_reqwest_client, parse_http_announce_response};
use crate::tracker::types::TRACKER_ENCODE_SET;
use percent_encoding::percent_encode;

impl HttpTrackerClient {
    pub fn new(
        tracker_url: String,
        info_hash: [u8; 20],
        peer_id: [u8; 20],
        port: u16,
        proxy: Option<&ProxyConfig>,
    ) -> Self {
        let http = build_reqwest_client(proxy);
        Self { tracker_url, info_hash, peer_id, port, http }
    }

    pub async fn announce(
        &self,
        uploaded: u64,
        event: &str,
    ) -> Result<AnnounceResponse, reqwest::Error> {
        let info_hash_encoded = percent_encode(&self.info_hash, TRACKER_ENCODE_SET).to_string();
        let peer_id_encoded = percent_encode(&self.peer_id, TRACKER_ENCODE_SET).to_string();
        let query = format!(
            "info_hash={}&peer_id={}&port={}&uploaded={}&downloaded=0&left=0\
             &compact=1&event={}&numwant=50",
            info_hash_encoded, peer_id_encoded, self.port, uploaded, event
        );
        let url = format!("{}?{}", self.tracker_url, query);
        log::debug!("[Tracker/HTTP] GET {}", url);
        let resp = self.http.get(&url).send().await?;
        let body = resp.bytes().await?;
        Ok(parse_http_announce_response(&body))
    }
}