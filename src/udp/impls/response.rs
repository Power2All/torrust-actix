use crate::udp::enums::response::Response;
use crate::udp::structs::announce_response::AnnounceResponse;
use crate::udp::structs::connect_response::ConnectResponse;
use crate::udp::structs::error_response::ErrorResponse;
use crate::udp::structs::scrape_response::ScrapeResponse;
use std::io;
use std::io::Write;
use std::net::{
    Ipv4Addr,
    Ipv6Addr
};

impl From<ConnectResponse> for Response {
    fn from(r: ConnectResponse) -> Self {
        Self::Connect(r)
    }
}

impl From<AnnounceResponse<Ipv4Addr>> for Response {
    fn from(r: AnnounceResponse<Ipv4Addr>) -> Self {
        Self::AnnounceIpv4(r)
    }
}

impl From<AnnounceResponse<Ipv6Addr>> for Response {
    fn from(r: AnnounceResponse<Ipv6Addr>) -> Self {
        Self::AnnounceIpv6(r)
    }
}

impl From<ScrapeResponse> for Response {
    fn from(r: ScrapeResponse) -> Self {
        Self::Scrape(r)
    }
}

impl From<ErrorResponse> for Response {
    fn from(r: ErrorResponse) -> Self {
        Self::Error(r)
    }
}

impl Response {
    /// Serialises the response into BEP 15 wire format.
    ///
    /// # Errors
    ///
    /// Returns the underlying I/O error when writing fails.
    #[inline]
    pub fn write(&self, bytes: &mut impl Write) -> Result<(), io::Error> {
        match self {
            Response::Connect(r) => {
                bytes.write_all(&0i32.to_be_bytes())?;
                bytes.write_all(&r.transaction_id.0.to_be_bytes())?;
                bytes.write_all(&r.connection_id.0.to_be_bytes())?;
            }
            Response::AnnounceIpv4(r) => {
                bytes.write_all(&1i32.to_be_bytes())?;
                bytes.write_all(&r.transaction_id.0.to_be_bytes())?;
                bytes.write_all(&r.announce_interval.0.to_be_bytes())?;
                bytes.write_all(&r.leechers.0.to_be_bytes())?;
                bytes.write_all(&r.seeders.0.to_be_bytes())?;
                let peer_count = r.peers.len();
                if peer_count > 0 {
                    let mut peer_buffer = Vec::with_capacity(peer_count * 6);
                    for peer in &r.peers {
                        peer_buffer.extend_from_slice(&peer.ip_address.octets());
                        peer_buffer.extend_from_slice(&peer.port.0.to_be_bytes());
                    }
                    bytes.write_all(&peer_buffer)?;
                }
            }
            Response::AnnounceIpv6(r) => {
                bytes.write_all(&1i32.to_be_bytes())?;
                bytes.write_all(&r.transaction_id.0.to_be_bytes())?;
                bytes.write_all(&r.announce_interval.0.to_be_bytes())?;
                bytes.write_all(&r.leechers.0.to_be_bytes())?;
                bytes.write_all(&r.seeders.0.to_be_bytes())?;
                let peer_count = r.peers.len();
                if peer_count > 0 {
                    let mut peer_buffer = Vec::with_capacity(peer_count * 18);
                    for peer in &r.peers {
                        peer_buffer.extend_from_slice(&peer.ip_address.octets());
                        peer_buffer.extend_from_slice(&peer.port.0.to_be_bytes());
                    }
                    bytes.write_all(&peer_buffer)?;
                }
            }
            Response::Scrape(r) => {
                bytes.write_all(&2i32.to_be_bytes())?;
                bytes.write_all(&r.transaction_id.0.to_be_bytes())?;
                let stats_count = r.torrent_stats.len();
                if stats_count > 0 {
                    let mut stats_buffer = Vec::with_capacity(stats_count * 12);
                    for torrent_stat in &r.torrent_stats {
                        stats_buffer.extend_from_slice(&torrent_stat.seeders.0.to_be_bytes());
                        stats_buffer.extend_from_slice(&torrent_stat.completed.0.to_be_bytes());
                        stats_buffer.extend_from_slice(&torrent_stat.leechers.0.to_be_bytes());
                    }
                    bytes.write_all(&stats_buffer)?;
                }
            }
            Response::Error(r) => {
                bytes.write_all(&3i32.to_be_bytes())?;
                bytes.write_all(&r.transaction_id.0.to_be_bytes())?;
                bytes.write_all(r.message.as_bytes())?;
            }
        }
        Ok(())
    }

    /// Returns the exact encoded size in bytes, for pre-allocating the output buffer.
    #[inline]
    pub fn estimated_size(&self) -> usize {
        match self {
            Response::Connect(_) => 16,
            Response::AnnounceIpv4(r) => 20 + (r.peers.len() * 6),
            Response::AnnounceIpv6(r) => 20 + (r.peers.len() * 18),
            Response::Scrape(r) => 8 + (r.torrent_stats.len() * 12),
            Response::Error(r) => 8 + r.message.len(),
        }
    }

    /// Serialises the response into a freshly allocated, exactly sized buffer.
    ///
    /// # Errors
    ///
    /// Returns the underlying I/O error when writing fails.
    #[inline]
    pub fn write_to_vec(&self) -> Result<Vec<u8>, io::Error> {
        let estimated_size = self.estimated_size();
        let mut buffer = Vec::with_capacity(estimated_size);
        self.write(&mut buffer)?;
        Ok(buffer)
    }
}