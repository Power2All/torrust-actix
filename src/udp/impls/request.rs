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

    pub fn from_bytes(bytes: &[u8], max_scrape_torrents: u8) -> Result<Self, RequestParseError> {
        let mut cursor = Cursor::new(bytes);

        let connection_id = cursor
            .read_i64::<NetworkEndian>()
            .map_err(RequestParseError::unsendable_io)?;
        let action = cursor
            .read_i32::<NetworkEndian>()
            .map_err(RequestParseError::unsendable_io)?;
        let transaction_id = cursor
            .read_i32::<NetworkEndian>()
            .map_err(RequestParseError::unsendable_io)?;

        match action {
            // Connect
            0 => {
                if connection_id == PROTOCOL_IDENTIFIER {
                    Ok((ConnectRequest {
                        transaction_id: TransactionId(transaction_id),
                    })
                        .into())
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

                cursor.read_exact(&mut info_hash).map_err(|err| {
                    RequestParseError::sendable_io(err, connection_id, transaction_id)
                })?;
                cursor.read_exact(&mut peer_id).map_err(|err| {
                    RequestParseError::sendable_io(err, connection_id, transaction_id)
                })?;

                let bytes_downloaded = cursor.read_i64::<NetworkEndian>().map_err(|err| {
                    RequestParseError::sendable_io(err, connection_id, transaction_id)
                })?;
                let bytes_left = cursor.read_i64::<NetworkEndian>().map_err(|err| {
                    RequestParseError::sendable_io(err, connection_id, transaction_id)
                })?;
                let bytes_uploaded = cursor.read_i64::<NetworkEndian>().map_err(|err| {
                    RequestParseError::sendable_io(err, connection_id, transaction_id)
                })?;
                let event = cursor.read_i32::<NetworkEndian>().map_err(|err| {
                    RequestParseError::sendable_io(err, connection_id, transaction_id)
                })?;

                cursor.read_exact(&mut ip).map_err(|err| {
                    RequestParseError::sendable_io(err, connection_id, transaction_id)
                })?;

                let key = cursor.read_u32::<NetworkEndian>().map_err(|err| {
                    RequestParseError::sendable_io(err, connection_id, transaction_id)
                })?;
                let peers_wanted = cursor.read_i32::<NetworkEndian>().map_err(|err| {
                    RequestParseError::sendable_io(err, connection_id, transaction_id)
                })?;
                let port = cursor.read_u16::<NetworkEndian>().map_err(|err| {
                    RequestParseError::sendable_io(err, connection_id, transaction_id)
                })?;

                let opt_ip = if ip == [0; 4] {
                    None
                } else {
                    Some(Ipv4Addr::from(ip))
                };

                let option_byte = cursor.read_u8();
                let option_size = cursor.read_u8();
                let mut path: &str = "";
                let mut path_array = vec![];

                let option_byte_value = option_byte.unwrap_or_default();
                let option_size_value = option_size.unwrap_or_default();
                if option_byte_value == 2 {
                    path_array.resize(option_size_value as usize, 0u8);
                    cursor.read_exact(&mut path_array).map_err(|err| {
                        RequestParseError::sendable_io(err, connection_id, transaction_id)
                    })?;
                    path = std::str::from_utf8(&path_array).unwrap_or_default();
                }

                Ok((AnnounceRequest {
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
                    path: path.to_string(),
                })
                    .into())
            }

            // Scrape
            2 => {
                let position = cursor.position() as usize;
                let inner = cursor.into_inner();

                let info_hashes: Vec<InfoHash> = inner[position..]
                    .chunks_exact(20)
                    .take(max_scrape_torrents as usize)
                    .map(|chunk| InfoHash(chunk.try_into().unwrap()))
                    .collect();

                if info_hashes.is_empty() {
                    Err(RequestParseError::sendable_text(
                        "Full scrapes are not allowed",
                        connection_id,
                        transaction_id,
                    ))
                } else {
                    Ok((ScrapeRequest {
                        connection_id: ConnectionId(connection_id),
                        transaction_id: TransactionId(transaction_id),
                        info_hashes,
                    })
                        .into())
                }
            }

            _ => Err(RequestParseError::sendable_text(
                "Invalid action",
                connection_id,
                transaction_id,
            )),
        }
    }
}