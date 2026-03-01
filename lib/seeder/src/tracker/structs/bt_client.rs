use crate::tracker::structs::http_client::BtHttpClient;
use crate::tracker::structs::udp_client::BtUdpClient;

#[derive(Clone)]
pub enum BtTrackerClient {
    Http(BtHttpClient),
    Udp(BtUdpClient),
}