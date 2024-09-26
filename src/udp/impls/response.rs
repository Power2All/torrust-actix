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

                for peer in r.peers.iter() {
                    bytes.write_all(&peer.ip_address.octets())?;
                    bytes.write_u16::<NetworkEndian>(peer.port.0)?;
                }
            }
            Response::AnnounceIpv6(r) => {
                bytes.write_i32::<NetworkEndian>(1)?;
                bytes.write_i32::<NetworkEndian>(r.transaction_id.0)?;
                bytes.write_i32::<NetworkEndian>(r.announce_interval.0)?;
                bytes.write_i32::<NetworkEndian>(r.leechers.0)?;
                bytes.write_i32::<NetworkEndian>(r.seeders.0)?;

                for peer in r.peers.iter() {
                    bytes.write_all(&peer.ip_address.octets())?;
                    bytes.write_u16::<NetworkEndian>(peer.port.0)?;
                }
            }
            Response::Scrape(r) => {
                bytes.write_i32::<NetworkEndian>(2)?;
                bytes.write_i32::<NetworkEndian>(r.transaction_id.0)?;

                for torrent_stat in r.torrent_stats.iter() {
                    bytes.write_i32::<NetworkEndian>(torrent_stat.seeders.0)?;
                    bytes.write_i32::<NetworkEndian>(torrent_stat.completed.0)?;
                    bytes.write_i32::<NetworkEndian>(torrent_stat.leechers.0)?;
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

    #[inline]
    pub fn from_bytes(bytes: &[u8], ipv4: bool) -> Result<Self, io::Error> {
        let mut cursor = Cursor::new(bytes);

        let action = cursor.read_i32::<NetworkEndian>()?;
        let transaction_id = cursor.read_i32::<NetworkEndian>()?;

        match action {
            // Connect
            0 => {
                let connection_id = cursor.read_i64::<NetworkEndian>()?;

                Ok((ConnectResponse {
                    connection_id: ConnectionId(connection_id),
                    transaction_id: TransactionId(transaction_id),
                })
                    .into())
            }
            // Announce
            1 if ipv4 => {
                let announce_interval = cursor.read_i32::<NetworkEndian>()?;
                let leechers = cursor.read_i32::<NetworkEndian>()?;
                let seeders = cursor.read_i32::<NetworkEndian>()?;

                let position = cursor.position() as usize;
                let inner = cursor.into_inner();

                let peers = inner[position..]
                    .chunks_exact(6)
                    .map(|chunk| {
                        let ip_bytes: [u8; 4] = (&chunk[..4]).try_into().unwrap();
                        let ip_address = Ipv4Addr::from(ip_bytes);
                        let port = (&chunk[4..]).read_u16::<NetworkEndian>().unwrap();

                        ResponsePeer {
                            ip_address,
                            port: Port(port),
                        }
                    })
                    .collect();

                Ok((AnnounceResponse {
                    transaction_id: TransactionId(transaction_id),
                    announce_interval: AnnounceInterval(announce_interval),
                    leechers: NumberOfPeers(leechers),
                    seeders: NumberOfPeers(seeders),
                    peers,
                })
                    .into())
            }
            1 if !ipv4 => {
                let announce_interval = cursor.read_i32::<NetworkEndian>()?;
                let leechers = cursor.read_i32::<NetworkEndian>()?;
                let seeders = cursor.read_i32::<NetworkEndian>()?;

                let position = cursor.position() as usize;
                let inner = cursor.into_inner();

                let peers = inner[position..]
                    .chunks_exact(18)
                    .map(|chunk| {
                        let ip_bytes: [u8; 16] = (&chunk[..16]).try_into().unwrap();
                        let ip_address = Ipv6Addr::from(ip_bytes);
                        let port = (&chunk[16..]).read_u16::<NetworkEndian>().unwrap();

                        ResponsePeer {
                            ip_address,
                            port: Port(port),
                        }
                    })
                    .collect();

                Ok((AnnounceResponse {
                    transaction_id: TransactionId(transaction_id),
                    announce_interval: AnnounceInterval(announce_interval),
                    leechers: NumberOfPeers(leechers),
                    seeders: NumberOfPeers(seeders),
                    peers,
                })
                    .into())
            }
            // Scrape
            2 => {
                let position = cursor.position() as usize;
                let inner = cursor.into_inner();

                let stats = inner[position..]
                    .chunks_exact(12)
                    .map(|chunk| {
                        let mut cursor: Cursor<&[u8]> = Cursor::new(chunk);

                        let seeders = cursor.read_i32::<NetworkEndian>().unwrap();
                        let downloads = cursor.read_i32::<NetworkEndian>().unwrap();
                        let leechers = cursor.read_i32::<NetworkEndian>().unwrap();

                        TorrentScrapeStatistics {
                            seeders: NumberOfPeers(seeders),
                            completed: NumberOfDownloads(downloads),
                            leechers: NumberOfPeers(leechers),
                        }
                    })
                    .collect();

                Ok((ScrapeResponse {
                    transaction_id: TransactionId(transaction_id),
                    torrent_stats: stats,
                })
                    .into())
            }
            // Error
            3 => {
                let position = cursor.position() as usize;
                let inner = cursor.into_inner();

                Ok((ErrorResponse {
                    transaction_id: TransactionId(transaction_id),
                    message: String::from_utf8_lossy(&inner[position..])
                        .into_owned()
                        .into(),
                })
                    .into())
            }
            _ => Ok((ErrorResponse {
                transaction_id: TransactionId(transaction_id),
                message: "Invalid action".into(),
            })
                .into()),
        }
    }
}