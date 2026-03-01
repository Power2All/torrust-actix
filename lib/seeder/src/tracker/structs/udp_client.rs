#[derive(Clone)]
pub struct BtUdpClient {
    pub tracker_url: String,
    pub info_hash: [u8; 20],
    pub peer_id: [u8; 20],
    pub port: u16,
}