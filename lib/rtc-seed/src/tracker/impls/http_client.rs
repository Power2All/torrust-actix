use crate::tracker::structs::announce_response::AnnounceResponse;
use crate::tracker::structs::http_client::TrackerClient;
use crate::tracker::tracker::parse_announce_response;
use crate::tracker::types::TRACKER_ENCODE_SET;
use percent_encoding::percent_encode;

impl TrackerClient {
    pub fn new(tracker_url: String, info_hash: [u8; 20], peer_id: [u8; 20]) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .expect("failed to build reqwest client");
        Self { tracker_url, info_hash, peer_id, http }
    }

    pub async fn announce_seeder(
        &self,
        sdp_offer: &str,
        uploaded: u64,
        event: &str,
    ) -> Result<AnnounceResponse, reqwest::Error> {
        let info_hash_encoded =
            percent_encode(&self.info_hash, TRACKER_ENCODE_SET).to_string();
        let peer_id_encoded =
            percent_encode(&self.peer_id, TRACKER_ENCODE_SET).to_string();
        let sdp_encoded = percent_encoding::utf8_percent_encode(sdp_offer, TRACKER_ENCODE_SET)
            .to_string();
        let query = format!(
            "info_hash={}&peer_id={}&port=6881&uploaded={}&downloaded=0&left=0\
             &compact=1&event={}&numwant=50&rtctorrent=1&rtcoffer={}",
            info_hash_encoded, peer_id_encoded, uploaded, event, sdp_encoded
        );
        let url = format!("{}?{}", self.tracker_url, query);
        log::debug!("[Tracker] GET {}", url);
        let resp = self.http.get(&url).send().await?;
        let body = resp.bytes().await?;
        Ok(parse_announce_response(&body))
    }
}