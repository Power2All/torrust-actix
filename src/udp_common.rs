use std::borrow::Cow;
use std::convert::TryInto;
use std::io::{self, Cursor, Read, Write};
use byteorder::{NetworkEndian, BigEndian, ReadBytesExt, WriteBytesExt};
use either::Either;
use std::fmt::Debug;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::SystemTime;
use thiserror::Error;

const PROTOCOL_IDENTIFIER: i64 = 4_497_486_125_440;

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum AnnounceEvent {
    Started,
    Stopped,
    Completed,
    None,
}

impl AnnounceEvent {
    #[inline]
    pub fn from_i32(i: i32) -> Self {
        match i {
            1 => Self::Completed,
            2 => Self::Started,
            3 => Self::Stopped,
            _ => Self::None,
        }
    }

    #[inline]
    pub fn to_i32(&self) -> i32 {
        match self {
            AnnounceEvent::None => 0,
            AnnounceEvent::Completed => 1,
            AnnounceEvent::Started => 2,
            AnnounceEvent::Stopped => 3,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ConnectRequest {
    pub transaction_id: TransactionId,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct AnnounceRequest {
    pub connection_id: ConnectionId,
    pub transaction_id: TransactionId,
    pub info_hash: InfoHash,
    pub peer_id: PeerId,
    pub bytes_downloaded: NumberOfBytes,
    pub bytes_uploaded: NumberOfBytes,
    pub bytes_left: NumberOfBytes,
    pub event: AnnounceEvent,
    pub ip_address: Option<Ipv4Addr>,
    pub key: PeerKey,
    pub peers_wanted: NumberOfPeers,
    pub port: Port,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ScrapeRequest {
    pub connection_id: ConnectionId,
    pub transaction_id: TransactionId,
    pub info_hashes: Vec<InfoHash>,
}

#[derive(Debug)]
pub enum RequestParseError {
    Sendable {
        connection_id: ConnectionId,
        transaction_id: TransactionId,
        err: Either<io::Error, &'static str>,
    },
    Unsendable {
        err: Either<io::Error, &'static str>,
    },
}

impl RequestParseError {
    pub fn sendable_io(err: io::Error, connection_id: i64, transaction_id: i32) -> Self {
        Self::Sendable {
            connection_id: ConnectionId(connection_id),
            transaction_id: TransactionId(transaction_id),
            err: Either::Left(err),
        }
    }
    pub fn sendable_text(text: &'static str, connection_id: i64, transaction_id: i32) -> Self {
        Self::Sendable {
            connection_id: ConnectionId(connection_id),
            transaction_id: TransactionId(transaction_id),
            err: Either::Right(text),
        }
    }
    pub fn unsendable_io(err: io::Error) -> Self {
        Self::Unsendable {
            err: Either::Left(err),
        }
    }
    pub fn unsendable_text(text: &'static str) -> Self {
        Self::Unsendable {
            err: Either::Right(text),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Request {
    Connect(ConnectRequest),
    Announce(AnnounceRequest),
    Scrape(ScrapeRequest),
}

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
                })
                    .into())
            }

            // Scrape
            2 => {
                let position = cursor.position() as usize;
                let inner = cursor.into_inner();

                let info_hashes: Vec<InfoHash> = (&inner[position..])
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

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub struct TorrentScrapeStatistics {
    pub seeders: NumberOfPeers,
    pub completed: NumberOfDownloads,
    pub leechers: NumberOfPeers,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ConnectResponse {
    pub connection_id: ConnectionId,
    pub transaction_id: TransactionId,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct AnnounceResponse<I: Ip> {
    pub transaction_id: TransactionId,
    pub announce_interval: AnnounceInterval,
    pub leechers: NumberOfPeers,
    pub seeders: NumberOfPeers,
    pub peers: Vec<ResponsePeer<I>>,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ScrapeResponse {
    pub transaction_id: TransactionId,
    pub torrent_stats: Vec<TorrentScrapeStatistics>,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ErrorResponse {
    pub transaction_id: TransactionId,
    pub message: Cow<'static, str>,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Response {
    Connect(ConnectResponse),
    AnnounceIpv4(AnnounceResponse<Ipv4Addr>),
    AnnounceIpv6(AnnounceResponse<Ipv6Addr>),
    Scrape(ScrapeResponse),
    Error(ErrorResponse),
}

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

pub trait Ip: Clone + Copy + Debug + PartialEq + Eq {}

impl Ip for Ipv4Addr {}
impl Ip for Ipv6Addr {}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct AnnounceInterval(pub i32);

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct InfoHash(pub [u8; 20]);

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct ConnectionId(pub i64);

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct TransactionId(pub i32);

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct NumberOfBytes(pub i64);

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct NumberOfPeers(pub i32);

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct NumberOfDownloads(pub i32);

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct Port(pub u16);

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, PartialOrd, Ord)]
pub struct PeerId(pub [u8; 20]);

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct PeerKey(pub u32);

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ResponsePeer<I: Ip> {
    pub ip_address: I,
    pub port: Port,
}

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("internal server error")]
    InternalServerError,

    #[error("info_hash is either missing or invalid")]
    InvalidInfoHash,

    #[error("info_hash unknown")]
    UnknownInfoHash,

    #[error("could not find remote address")]
    AddressNotFound,

    #[error("torrent has no peers")]
    NoPeersFound,

    #[error("torrent not on whitelist")]
    TorrentNotWhitelisted,

    #[error("torrent blacklist")]
    TorrentBlacklisted,

    #[error("peer not authenticated")]
    PeerNotAuthenticated,

    #[error("invalid authentication key")]
    PeerKeyNotValid,

    #[error("exceeded info_hash limit")]
    ExceededInfoHashLimit,

    #[error("bad request")]
    BadRequest,
}

pub fn get_connection_id(remote_address: &SocketAddr) -> ConnectionId {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => ConnectionId(((duration.as_secs() / 3600) | ((remote_address.port() as u64) << 36)) as i64),
        Err(_) => ConnectionId(0x7FFFFFFFFFFFFFFF),
    }
}

pub fn current_time() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH).unwrap()
        .as_secs()
}

// Function that will convert a small or big number into the smallest form of a byte array.
pub async fn convert_int_to_bytes(number: &u64) -> Vec<u8> {
    let mut return_data: Vec<u8> = Vec::new();
    // return_data.extend(number.to_be_bytes().reverse());
    for i in 1..8 {
        if number < &256u64.pow(i) {
            let start: usize = 16usize - i as usize;
            return_data.extend(number.to_be_bytes()[start..8].iter());
            return return_data;
        }
    }
    return_data
}

pub async fn convert_bytes_to_int(array: &Vec<u8>) -> u64 {
    let mut array_fixed: Vec<u8> = Vec::new();
    let size = 8 - array.len();
    array_fixed.resize(size, 0);
    array_fixed.extend(array);
    let mut rdr = Cursor::new(array_fixed);
    rdr.read_u64::<BigEndian>().unwrap()
}
