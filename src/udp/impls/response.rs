use crate::udp::enums::response::Response;
use crate::udp::structs::announce_interval::AnnounceInterval;
use crate::udp::structs::announce_response::AnnounceResponse;
use crate::udp::structs::connect_response::ConnectResponse;
use crate::udp::structs::connection_id::ConnectionId;
use crate::udp::structs::error_response::ErrorResponse;
use crate::udp::structs::number_of_peers::NumberOfPeers;
use crate::udp::structs::scrape_response::ScrapeResponse;
use crate::udp::structs::transaction_id::TransactionId;
use crate::udp::udp::{
    parse_ipv4_peers,
    parse_ipv6_peers,
    parse_scrape_stats,
    read_be
};
use std::io;
use std::io::{
    Cursor,
    Write
};
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

    /// Parses a BEP 15 response datagram (connect, announce with `ipv4` peer format flag,
    /// scrape or error).
    ///
    /// # Errors
    ///
    /// Returns an I/O error when the datagram is truncated. An unknown action is not an
    /// error: it yields an `ErrorResponse` with the message `Invalid action`.
    #[inline]
    pub fn from_bytes(bytes: &[u8], ipv4: bool) -> Result<Self, io::Error> {
        let mut cursor = Cursor::new(bytes);
        let action = i32::from_be_bytes(read_be(&mut cursor)?);
        let transaction_id = i32::from_be_bytes(read_be(&mut cursor)?);
        match action {
            0 => {
                let connection_id = i64::from_be_bytes(read_be(&mut cursor)?);
                Ok(ConnectResponse {
                    connection_id: ConnectionId(connection_id),
                    transaction_id: TransactionId(transaction_id),
                }
                    .into())
            }
            1 => {
                let announce_interval = i32::from_be_bytes(read_be(&mut cursor)?);
                let leechers = i32::from_be_bytes(read_be(&mut cursor)?);
                let seeders = i32::from_be_bytes(read_be(&mut cursor)?);
                let position = cursor.position() as usize;
                let remaining_bytes = &bytes[position..];
                if ipv4 {
                    let peers = parse_ipv4_peers(remaining_bytes)?;
                    Ok(AnnounceResponse {
                        transaction_id: TransactionId(transaction_id),
                        announce_interval: AnnounceInterval(announce_interval),
                        leechers: NumberOfPeers(leechers),
                        seeders: NumberOfPeers(seeders),
                        peers,
                    }
                        .into())
                } else {
                    let peers = parse_ipv6_peers(remaining_bytes)?;
                    Ok(AnnounceResponse {
                        transaction_id: TransactionId(transaction_id),
                        announce_interval: AnnounceInterval(announce_interval),
                        leechers: NumberOfPeers(leechers),
                        seeders: NumberOfPeers(seeders),
                        peers,
                    }
                        .into())
                }
            }
            2 => {
                let position = cursor.position() as usize;
                let remaining_bytes = &bytes[position..];
                let torrent_stats = parse_scrape_stats(remaining_bytes)?;
                Ok(ScrapeResponse {
                    transaction_id: TransactionId(transaction_id),
                    torrent_stats,
                }
                    .into())
            }
            3 => {
                let position = cursor.position() as usize;
                let message_bytes = &bytes[position..];
                let message = String::from_utf8_lossy(message_bytes).into_owned();
                Ok(ErrorResponse {
                    transaction_id: TransactionId(transaction_id),
                    message: message.into(),
                }
                    .into())
            }
            _ => Ok(ErrorResponse {
                transaction_id: TransactionId(transaction_id),
                message: "Invalid action".into(),
            }
                .into()),
        }
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