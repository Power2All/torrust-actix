use crate::config::structs::seeder_config::SeederConfig;
use crate::seeder::seeder::{
    setup_handlers,
    SharedRateLimiter
};
use crate::seeder::structs::peer_conn::PeerConn;
use crate::torrent::structs::torrent_info::TorrentInfo;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use webrtc::api::APIBuilder;
use webrtc::data_channel::data_channel_init::RTCDataChannelInit;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

impl PeerConn {
    pub async fn new(
        config: &SeederConfig,
        torrent_info: Arc<TorrentInfo>,
        uploaded: Arc<AtomicU64>,
        rate_limiter: Option<SharedRateLimiter>,
    ) -> Result<Self, webrtc::Error> {
        let ice_servers: Vec<RTCIceServer> = config
            .ice_servers
            .iter()
            .map(|url| RTCIceServer {
                urls: vec![url.clone()],
                ..Default::default()
            })
            .collect();
        let rtc_config = RTCConfiguration {
            ice_servers,
            ..Default::default()
        };
        let api = APIBuilder::new().build();
        let peer_connection = Arc::new(api.new_peer_connection(rtc_config).await?);
        let mut gather_complete = peer_connection.gathering_complete_promise().await;
        let dc_init = RTCDataChannelInit {
            ordered: Some(false),
            max_retransmits: Some(3),
            ..Default::default()
        };
        let data_channel = peer_connection
            .create_data_channel("torrent", Some(dc_init))
            .await?;
        setup_handlers(Arc::clone(&data_channel), Arc::clone(&torrent_info), Arc::clone(&uploaded), rate_limiter);
        let offer = peer_connection.create_offer(None).await?;
        peer_connection.set_local_description(offer).await?;
        tokio::select! {
            _ = gather_complete.recv() => {
                log::debug!("[PeerConn] ICE gathering complete");
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                log::debug!("[PeerConn] ICE gathering timed out — using current candidates");
            }
        }
        let local_desc = peer_connection
            .local_description()
            .await
            .ok_or(webrtc::Error::ErrNoRemoteDescription)?;
        let sdp_offer = local_desc.sdp.clone();
        log::info!("[PeerConn] Offer ready ({} bytes)", sdp_offer.len());
        Ok(PeerConn {
            peer_connection,
            data_channel,
            sdp_offer,
        })
    }

    pub async fn handle_answer(&self, sdp_answer: String) -> Result<(), webrtc::Error> {
        let answer = RTCSessionDescription::answer(sdp_answer)?;
        self.peer_connection.set_remote_description(answer).await?;
        log::info!("[PeerConn] Remote description set — handshake complete");
        Ok(())
    }
}