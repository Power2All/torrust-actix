use crate::tracker::structs::http_client::HttpTrackerClient;
use crate::tracker::structs::udp_client::UdpTrackerClient;

#[derive(Clone)]
pub enum TrackerClient {
    Http(HttpTrackerClient),
    Udp(UdpTrackerClient),
}