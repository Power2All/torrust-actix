use std::io;
use std::io::{Cursor, Read, Write};
use std::net::Ipv4Addr;
use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use crate::common::structs::number_of_bytes::NumberOfBytes;
use crate::tracker::enums::announce_event::AnnounceEvent;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::peer_id::PeerId;
use crate::udp::enums::request::Request;
use crate::udp::enums::request_parse_error::RequestParseError;
use crate::udp::structs::announce_request::AnnounceRequest;
use crate::udp::structs::connect_request::ConnectRequest;
use crate::udp::structs::connection_id::ConnectionId;
use crate::udp::structs::number_of_peers::NumberOfPeers;
use crate::udp::structs::peer_key::PeerKey;
use crate::udp::structs::port::Port;
use crate::udp::structs::scrape_request::ScrapeRequest;
use crate::udp::structs::transaction_id::TransactionId;
use crate::udp::udp::PROTOCOL_IDENTIFIER;

impl From<ConnectRequest> for Request {
    fn from(r: ConnectRequest) -> Self {
        Self::Connect(r)
    }
}

impl From<AnnounceRequest> for Request {
    fn from(r: AnnounceRequest) -> Self {
        Self::Announce(r)
    }
}

impl From<ScrapeRequest> for Request {
    fn from(r: ScrapeRequest) -> Self {
        Self::Scrape(r)
    }
}

impl Request {
    #[tracing::instrument(skip(bytes), level = "debug")]
    pub fn write(self, bytes: &mut impl Write) -> Result<(), io::Error> {
        match self {
            Request::Connect(r) => {
                bytes.write_i64::<NetworkEndian>(PROTOCOL_IDENTIFIER)?;
                bytes.write_i32::<NetworkEndian>(0)?;
                bytes.write_i32::<NetworkEndian>(r.transaction_id.0)?;
            }

            Request::Announce(r) => {
                bytes.write_i64::<NetworkEndian>(r.connection_id.0)?;
                bytes.write_i32::<NetworkEndian>(1)?;
                bytes.write_i32::<NetworkEndian>(r.transaction_id.0)?;

                bytes.write_all(&r.info_hash.0)?;
                bytes.write_all(&r.peer_id.0)?;

                bytes.write_i64::<NetworkEndian>(r.bytes_downloaded.0)?;
                bytes.write_i64::<NetworkEndian>(r.bytes_left.0)?;
                bytes.write_i64::<NetworkEndian>(r.bytes_uploaded.0)?;

                bytes.write_i32::<NetworkEndian>(r.event.to_i32())?;

                bytes.write_all(&r.ip_address.map_or([0; 4], |ip| ip.octets()))?;

                bytes.write_u32::<NetworkEndian>(r.key.0)?;
                bytes.write_i32::<NetworkEndian>(r.peers_wanted.0)?;
                bytes.write_u16::<NetworkEndian>(r.port.0)?;
            }

            Request::Scrape(r) => {
                bytes.write_i64::<NetworkEndian>(r.connection_id.0)?;
                bytes.write_i32::<NetworkEndian>(2)?;
                bytes.write_i32::<NetworkEndian>(r.transaction_id.0)?;

                for info_hash in r.info_hashes {
                    bytes.write_all(&info_hash.0)?;
                }
            }
        }

        Ok(())
    }

    #[tracing::instrument(level = "debug")]
    pub fn from_bytes(bytes: &[u8], max_scrape_torrents: u8) -> Result<Self, RequestParseError> {
        if bytes.len() < 16 {
            return Err(RequestParseError::unsendable_text("Packet too short"));
        }

        let connection_id = i64::from_be_bytes(bytes[0..8].try_into().map_err(|_|
            RequestParseError::unsendable_io(io::Error::new(io::ErrorKind::InvalidData, "Invalid connection_id"))
        )?);

        let action = i32::from_be_bytes(bytes[8..12].try_into().map_err(|_|
            RequestParseError::unsendable_io(io::Error::new(io::ErrorKind::InvalidData, "Invalid action"))
        )?);

        let transaction_id = i32::from_be_bytes(bytes[12..16].try_into().map_err(|_|
            RequestParseError::unsendable_io(io::Error::new(io::ErrorKind::InvalidData, "Invalid transaction_id"))
        )?);

        if action == 0 {
            if connection_id == PROTOCOL_IDENTIFIER {
                return Ok(ConnectRequest {
                    transaction_id: TransactionId(transaction_id),
                }.into());
            } else {
                return Err(RequestParseError::unsendable_text("Protocol identifier missing"));
            }
        }

        let mut cursor = Cursor::new(bytes);
        cursor.set_position(16);

        match action {
            // Connect
            0 => {
                if connection_id == PROTOCOL_IDENTIFIER {
                    Ok(ConnectRequest {
                        transaction_id: TransactionId(transaction_id),
                    }.into())
                } else {
                    Err(RequestParseError::unsendable_text(
                        "Protocol identifier missing",
                    ))
                }
            }

            // Announce
            1 => {
                let mut info_hash = [0; 20];
                let mut peer_id = [0; 20];
                let mut ip = [0; 4];

                let sendable_err = |err: io::Error| {
                    RequestParseError::sendable_io(err, connection_id, transaction_id)
                };

                cursor.read_exact(&mut info_hash).map_err(sendable_err)?;
                cursor.read_exact(&mut peer_id).map_err(sendable_err)?;

                let bytes_downloaded = cursor.read_i64::<NetworkEndian>().map_err(sendable_err)?;
                let bytes_left = cursor.read_i64::<NetworkEndian>().map_err(sendable_err)?;
                let bytes_uploaded = cursor.read_i64::<NetworkEndian>().map_err(sendable_err)?;
                let event = cursor.read_i32::<NetworkEndian>().map_err(sendable_err)?;

                cursor.read_exact(&mut ip).map_err(sendable_err)?;

                let key = cursor.read_u32::<NetworkEndian>().map_err(sendable_err)?;
                let peers_wanted = cursor.read_i32::<NetworkEndian>().map_err(sendable_err)?;
                let port = cursor.read_u16::<NetworkEndian>().map_err(sendable_err)?;

                let opt_ip = if ip == [0; 4] {
                    None
                } else {
                    Some(Ipv4Addr::from(ip))
                };

                let path = if cursor.position() < bytes.len() as u64 {
                    let option_byte = cursor.read_u8().ok();
                    let option_size = cursor.read_u8().ok();

                    if option_byte == Some(2) {
                        if let Some(size) = option_size {
                            let size_usize = size as usize;
                            if cursor.position() + size_usize as u64 <= bytes.len() as u64 {
                                let start_pos = cursor.position() as usize;
                                let end_pos = start_pos + size_usize;
                                std::str::from_utf8(&bytes[start_pos..end_pos])
                                    .unwrap_or_default()
                                    .to_string()
                            } else {
                                String::new()
                            }
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };

                Ok(AnnounceRequest {
                    connection_id: ConnectionId(connection_id),
                    transaction_id: TransactionId(transaction_id),
                    info_hash: InfoHash(info_hash),
                    peer_id: PeerId(peer_id),
                    bytes_downloaded: NumberOfBytes(bytes_downloaded),
                    bytes_uploaded: NumberOfBytes(bytes_uploaded),
                    bytes_left: NumberOfBytes(bytes_left),
                    event: AnnounceEvent::from_i32(event),
                    ip_address: opt_ip,
                    key: PeerKey(key),
                    peers_wanted: NumberOfPeers(peers_wanted),
                    port: Port(port),
                    path,
                }.into())
            }

            // Scrape
            2 => {
                let position = cursor.position() as usize;
                let remaining_bytes = &bytes[position..];

                let max_hashes = max_scrape_torrents as usize;
                let available_hashes = remaining_bytes.len() / 20;
                let actual_hashes = available_hashes.min(max_hashes);

                if actual_hashes == 0 {
                    return Err(RequestParseError::sendable_text(
                        "Full scrapes are not allowed",
                        connection_id,
                        transaction_id,
                    ));
                }

                let mut info_hashes = Vec::with_capacity(actual_hashes);

                for chunk in remaining_bytes.chunks_exact(20).take(actual_hashes) {
                    let hash_array: [u8; 20] = chunk.try_into()
                        .map_err(|_| RequestParseError::sendable_text(
                            "Invalid info hash format",
                            connection_id,
                            transaction_id,
                        ))?;
                    info_hashes.push(InfoHash(hash_array));
                }

                Ok(ScrapeRequest {
                    connection_id: ConnectionId(connection_id),
                    transaction_id: TransactionId(transaction_id),
                    info_hashes,
                }.into())
            }

            _ => Err(RequestParseError::sendable_text(
                "Invalid action",
                connection_id,
                transaction_id,
            )),
        }
    }
}