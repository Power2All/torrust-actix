use std::convert::TryInto;
use std::io;
use std::io::{Cursor, Write};
use std::net::{Ipv4Addr, Ipv6Addr};
use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use crate::udp::enums::response::Response;
use crate::udp::structs::announce_interval::AnnounceInterval;
use crate::udp::structs::announce_response::AnnounceResponse;
use crate::udp::structs::connect_response::ConnectResponse;
use crate::udp::structs::connection_id::ConnectionId;
use crate::udp::structs::error_response::ErrorResponse;
use crate::udp::structs::number_of_downloads::NumberOfDownloads;
use crate::udp::structs::number_of_peers::NumberOfPeers;
use crate::udp::structs::port::Port;
use crate::udp::structs::response_peer::ResponsePeer;
use crate::udp::structs::scrape_response::ScrapeResponse;
use crate::udp::structs::torrent_scrape_statistics::TorrentScrapeStatistics;
use crate::udp::structs::transaction_id::TransactionId;

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
    #[tracing::instrument(skip(bytes), level = "debug")]
    #[inline]
    pub fn write(&self, bytes: &mut impl Write) -> Result<(), io::Error> {
        match self {
            Response::Connect(r) => {
                bytes.write_i32::<NetworkEndian>(0)?;
                bytes.write_i32::<NetworkEndian>(r.transaction_id.0)?;
                bytes.write_i64::<NetworkEndian>(r.connection_id.0)?;
            }
            Response::AnnounceIpv4(r) => {
                bytes.write_i32::<NetworkEndian>(1)?;
                bytes.write_i32::<NetworkEndian>(r.transaction_id.0)?;
                bytes.write_i32::<NetworkEndian>(r.announce_interval.0)?;
                bytes.write_i32::<NetworkEndian>(r.leechers.0)?;
                bytes.write_i32::<NetworkEndian>(r.seeders.0)?;

                
                let peer_count = r.peers.len();
                if peer_count > 0 {
                    let mut peer_buffer = Vec::with_capacity(peer_count * 6);
                    for peer in &r.peers {
                        peer_buffer.extend_from_slice(&peer.ip_address.octets());
                        peer_buffer.write_u16::<NetworkEndian>(peer.port.0)?;
                    }
                    bytes.write_all(&peer_buffer)?;
                }
            }
            Response::AnnounceIpv6(r) => {
                bytes.write_i32::<NetworkEndian>(1)?;
                bytes.write_i32::<NetworkEndian>(r.transaction_id.0)?;
                bytes.write_i32::<NetworkEndian>(r.announce_interval.0)?;
                bytes.write_i32::<NetworkEndian>(r.leechers.0)?;
                bytes.write_i32::<NetworkEndian>(r.seeders.0)?;

                
                let peer_count = r.peers.len();
                if peer_count > 0 {
                    let mut peer_buffer = Vec::with_capacity(peer_count * 18);
                    for peer in &r.peers {
                        peer_buffer.extend_from_slice(&peer.ip_address.octets());
                        peer_buffer.write_u16::<NetworkEndian>(peer.port.0)?;
                    }
                    bytes.write_all(&peer_buffer)?;
                }
            }
            Response::Scrape(r) => {
                bytes.write_i32::<NetworkEndian>(2)?;
                bytes.write_i32::<NetworkEndian>(r.transaction_id.0)?;

                
                let stats_count = r.torrent_stats.len();
                if stats_count > 0 {
                    let mut stats_buffer = Vec::with_capacity(stats_count * 12);
                    for torrent_stat in &r.torrent_stats {
                        stats_buffer.write_i32::<NetworkEndian>(torrent_stat.seeders.0)?;
                        stats_buffer.write_i32::<NetworkEndian>(torrent_stat.completed.0)?;
                        stats_buffer.write_i32::<NetworkEndian>(torrent_stat.leechers.0)?;
                    }
                    bytes.write_all(&stats_buffer)?;
                }
            }
            Response::Error(r) => {
                bytes.write_i32::<NetworkEndian>(3)?;
                bytes.write_i32::<NetworkEndian>(r.transaction_id.0)?;
                bytes.write_all(r.message.as_bytes())?;
            }
        }

        Ok(())
    }

    #[tracing::instrument(level = "debug")]
    #[inline]
    pub fn from_bytes(bytes: &[u8], ipv4: bool) -> Result<Self, io::Error> {
        let mut cursor = Cursor::new(bytes);

        let action = cursor.read_i32::<NetworkEndian>()?;
        let transaction_id = cursor.read_i32::<NetworkEndian>()?;

        match action {
            
            0 => {
                let connection_id = cursor.read_i64::<NetworkEndian>()?;

                Ok(ConnectResponse {
                    connection_id: ConnectionId(connection_id),
                    transaction_id: TransactionId(transaction_id),
                }
                    .into())
            }
            
            1 => {
                let announce_interval = cursor.read_i32::<NetworkEndian>()?;
                let leechers = cursor.read_i32::<NetworkEndian>()?;
                let seeders = cursor.read_i32::<NetworkEndian>()?;

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

    
    #[inline]
    pub fn write_to_vec(&self) -> Result<Vec<u8>, io::Error> {
        let estimated_size = self.estimated_size();
        let mut buffer = Vec::with_capacity(estimated_size);
        self.write(&mut buffer)?;
        Ok(buffer)
    }
}

#[inline]
fn parse_ipv4_peers(bytes: &[u8]) -> Result<Vec<ResponsePeer<Ipv4Addr>>, io::Error> {
    let chunk_size = 6;
    let peer_count = bytes.len() / chunk_size;
    let mut peers = Vec::with_capacity(peer_count);

    for chunk in bytes.chunks_exact(chunk_size) {
        let ip_bytes: [u8; 4] = chunk[..4].try_into().map_err(|_|
            io::Error::new(io::ErrorKind::InvalidData, "Invalid IPv4 address bytes")
        )?;

        let port = (&chunk[4..6]).read_u16::<NetworkEndian>().map_err(|e|
            io::Error::new(io::ErrorKind::InvalidData, e)
        )?;

        peers.push(ResponsePeer {
            ip_address: Ipv4Addr::from(ip_bytes),
            port: Port(port),
        });
    }

    Ok(peers)
}

#[inline]
fn parse_ipv6_peers(bytes: &[u8]) -> Result<Vec<ResponsePeer<Ipv6Addr>>, io::Error> {
    let chunk_size = 18;
    let peer_count = bytes.len() / chunk_size;
    let mut peers = Vec::with_capacity(peer_count);

    for chunk in bytes.chunks_exact(chunk_size) {
        let ip_bytes: [u8; 16] = chunk[..16].try_into().map_err(|_|
            io::Error::new(io::ErrorKind::InvalidData, "Invalid IPv6 address bytes")
        )?;

        let port = (&chunk[16..18]).read_u16::<NetworkEndian>().map_err(|e|
            io::Error::new(io::ErrorKind::InvalidData, e)
        )?;

        peers.push(ResponsePeer {
            ip_address: Ipv6Addr::from(ip_bytes),
            port: Port(port),
        });
    }

    Ok(peers)
}

#[inline]
fn parse_scrape_stats(bytes: &[u8]) -> Result<Vec<TorrentScrapeStatistics>, io::Error> {
    let chunk_size = 12;
    let stats_count = bytes.len() / chunk_size;
    let mut stats = Vec::with_capacity(stats_count);

    for chunk in bytes.chunks_exact(chunk_size) {
        let mut cursor = Cursor::new(chunk);

        let seeders = cursor.read_i32::<NetworkEndian>().map_err(|e|
            io::Error::new(io::ErrorKind::InvalidData, e)
        )?;
        let downloads = cursor.read_i32::<NetworkEndian>().map_err(|e|
            io::Error::new(io::ErrorKind::InvalidData, e)
        )?;
        let leechers = cursor.read_i32::<NetworkEndian>().map_err(|e|
            io::Error::new(io::ErrorKind::InvalidData, e)
        )?;

        stats.push(TorrentScrapeStatistics {
            seeders: NumberOfPeers(seeders),
            completed: NumberOfDownloads(downloads),
            leechers: NumberOfPeers(leechers),
        });
    }

    Ok(stats)
}