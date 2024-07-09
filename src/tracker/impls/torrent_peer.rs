use std::net::{IpAddr, SocketAddr};
use crate::tracker::structs::torrent_peer::TorrentPeer;

impl TorrentPeer {
    // pub fn from_udp_announce_request(announce_request: &AnnounceRequest, remote_ip: IpAddr) -> Self {
    //     let peer_addr = TorrentPeer::peer_addr_from_ip_and_port_and_opt_host_ip(remote_ip, announce_request.port.0);
    //
    //     let event = match announce_request.event {
    //         udp_common::AnnounceEvent::Started => { AnnounceEvent::Started }
    //         udp_common::AnnounceEvent::Stopped => { AnnounceEvent::Stopped }
    //         udp_common::AnnounceEvent::Completed => { AnnounceEvent::Completed }
    //         udp_common::AnnounceEvent::None => { AnnounceEvent::None }
    //     };
    //     TorrentPeer {
    //         peer_id: PeerId(announce_request.peer_id.0),
    //         peer_addr,
    //         peer_offer_id: None,
    //         peer_offer: None,
    //         updated: std::time::Instant::now(),
    //         uploaded: NumberOfBytes(announce_request.bytes_uploaded.0),
    //         downloaded: NumberOfBytes(announce_request.bytes_downloaded.0),
    //         left: NumberOfBytes(announce_request.bytes_left.0),
    //         event,
    //     }
    // }

    // potentially substitute localhost ip with external ip
    pub fn peer_addr_from_ip_and_port_and_opt_host_ip(remote_ip: IpAddr, port: u16) -> SocketAddr {
        SocketAddr::new(remote_ip, port)
    }
}
