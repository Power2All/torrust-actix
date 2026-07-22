use crate::config::enums::udp_receive_method::UdpReceiveMethod;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::udp::enums::simple_proxy_protocol::SppParseResult;
use crate::udp::structs::number_of_downloads::NumberOfDownloads;
use crate::udp::structs::number_of_peers::NumberOfPeers;
use crate::udp::structs::port::Port;
use crate::udp::structs::response_peer::ResponsePeer;
use crate::udp::structs::simple_proxy_protocol::SppHeader;
use crate::udp::structs::torrent_scrape_statistics::TorrentScrapeStatistics;
use crate::udp::structs::udp_server::UdpServer;
use log::{
    error,
    info
};
use std::io::{
    Error,
    ErrorKind,
    Read
};
use std::net::{
    IpAddr,
    Ipv4Addr,
    Ipv6Addr,
    SocketAddr
};
use std::process::exit;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;

pub const PROTOCOL_IDENTIFIER: i64 = 4_497_486_125_440;
pub const MAX_SCRAPE_TORRENTS: u8 = 74;
pub const MAX_PACKET_SIZE: usize = 1496;
pub const SPP_HEADER_SIZE: usize = 38;
pub const SPP_MAGIC: u16 = 0x56EC;

/// Reads exactly `N` big-endian bytes from `r`, for use with `iNN::from_be_bytes`.
///
/// # Errors
///
/// Returns an I/O error when `r` holds fewer than `N` bytes.
#[inline]
pub fn read_be<const N: usize>(r: &mut impl Read) -> Result<[u8; N], Error> {
    let mut buf = [0u8; N];
    r.read_exact(&mut buf)?;
    Ok(buf)
}

/// Spawns the UDP tracker service on `addr` using the selected receive backend and returns
/// its join handle. The service runs until the shutdown watch channel fires.
#[allow(clippy::too_many_arguments)]
pub async fn udp_service(addr: SocketAddr, udp_threads: usize, worker_threads: usize, recv_buffer_size: usize, send_buffer_size: usize, reuse_address: bool, use_payload_ip: bool, simple_proxy_protocol: bool, receive_method: UdpReceiveMethod, data: Arc<TorrentTracker>, rx: tokio::sync::watch::Receiver<bool>, tokio_udp: Arc<Runtime>) -> JoinHandle<()>
{
    let udp_server = UdpServer::new(data, addr, udp_threads, worker_threads, recv_buffer_size, send_buffer_size, reuse_address, use_payload_ip, simple_proxy_protocol, receive_method).await.unwrap_or_else(|e| {
        error!("Could not listen to the UDP port: {e}");
        exit(1);
    });
    let spp_status = if simple_proxy_protocol { " with Simple Proxy Protocol enabled" } else { "" };
    info!("[UDP] Starting a server listener on {addr} with {udp_threads} UDP threads and {worker_threads} worker threads{spp_status}");
    tokio_udp.spawn(async move {
        udp_server.start(rx).await;
    })
}

/// Parses packed 6-byte (IPv4 + port) peer entries from a UDP announce response body.
/// Trailing bytes that do not form a complete entry are ignored.
///
/// # Errors
///
/// Returns an I/O error when an entry cannot be decoded.
#[inline]
pub fn parse_ipv4_peers(bytes: &[u8]) -> Result<Vec<ResponsePeer<Ipv4Addr>>, Error> {
    let chunk_size = 6;
    let peer_count = bytes.len() / chunk_size;
    let mut peers = Vec::with_capacity(peer_count);
    for chunk in bytes.chunks_exact(chunk_size) {
        let ip_bytes: [u8; 4] = chunk[..4].try_into().map_err(|_|
            Error::new(ErrorKind::InvalidData, "Invalid IPv4 address bytes")
        )?;
        let port = u16::from_be_bytes([chunk[4], chunk[5]]);
        peers.push(ResponsePeer {
            ip_address: Ipv4Addr::from(ip_bytes),
            port: Port(port),
        });
    }
    Ok(peers)
}

/// Parses packed 18-byte (IPv6 + port) peer entries from a UDP announce response body.
/// Trailing bytes that do not form a complete entry are ignored.
///
/// # Errors
///
/// Returns an I/O error when an entry cannot be decoded.
#[inline]
pub fn parse_ipv6_peers(bytes: &[u8]) -> Result<Vec<ResponsePeer<Ipv6Addr>>, Error> {
    let chunk_size = 18;
    let peer_count = bytes.len() / chunk_size;
    let mut peers = Vec::with_capacity(peer_count);
    for chunk in bytes.chunks_exact(chunk_size) {
        let ip_bytes: [u8; 16] = chunk[..16].try_into().map_err(|_|
            Error::new(ErrorKind::InvalidData, "Invalid IPv6 address bytes")
        )?;
        let port = u16::from_be_bytes([chunk[16], chunk[17]]);
        peers.push(ResponsePeer {
            ip_address: Ipv6Addr::from(ip_bytes),
            port: Port(port),
        });
    }
    Ok(peers)
}

/// Parses packed 12-byte scrape statistics entries (seeders/completed/leechers).
/// Trailing bytes that do not form a complete entry are ignored.
///
/// # Errors
///
/// Returns an I/O error when an entry cannot be decoded.
#[inline]
pub fn parse_scrape_stats(bytes: &[u8]) -> Result<Vec<TorrentScrapeStatistics>, Error> {
    let chunk_size = 12;
    let stats_count = bytes.len() / chunk_size;
    let mut stats = Vec::with_capacity(stats_count);
    for chunk in bytes.chunks_exact(chunk_size) {
        let seeders = i32::from_be_bytes(chunk[0..4].try_into().expect("chunk is 12 bytes"));
        let downloads = i32::from_be_bytes(chunk[4..8].try_into().expect("chunk is 12 bytes"));
        let leechers = i32::from_be_bytes(chunk[8..12].try_into().expect("chunk is 12 bytes"));
        stats.push(TorrentScrapeStatistics {
            seeders: NumberOfPeers(seeders),
            completed: NumberOfDownloads(downloads),
            leechers: NumberOfPeers(leechers),
        });
    }
    Ok(stats)
}

/// Interprets 16 bytes as an IP address, collapsing IPv4-mapped IPv6 addresses to IPv4.
pub fn parse_address(bytes: &[u8; 16]) -> IpAddr {
    let is_ipv4_mapped = bytes[0..10] == [0u8; 10] && bytes[10] == 0xff && bytes[11] == 0xff;
    if is_ipv4_mapped {
        IpAddr::V4(Ipv4Addr::new(bytes[12], bytes[13], bytes[14], bytes[15]))
    } else {
        IpAddr::V6(Ipv6Addr::from(*bytes))
    }
}

/// Parses the Simple Proxy Protocol header prepended by supported UDP load balancers,
/// returning the real client address and the payload offset when present.
pub fn parse_spp_header(data: &[u8]) -> SppParseResult {
    if data.len() < 2 {
        return SppParseResult::NotPresent;
    }
    let magic = u16::from_be_bytes([data[0], data[1]]);
    if magic != SPP_MAGIC {
        return SppParseResult::NotPresent;
    }
    if data.len() < SPP_HEADER_SIZE {
        return SppParseResult::Malformed(format!(
            "SPP magic found but packet too small: {} bytes, need {}",
            data.len(),
            SPP_HEADER_SIZE
        ));
    }
    let client_addr_bytes: [u8; 16] = data[2..18]
        .try_into()
        .expect("slice with correct length");
    let client_addr = parse_address(&client_addr_bytes);
    let proxy_addr_bytes: [u8; 16] = data[18..34]
        .try_into()
        .expect("slice with correct length");
    let proxy_addr = parse_address(&proxy_addr_bytes);
    let client_port = u16::from_be_bytes([data[34], data[35]]);
    let proxy_port = u16::from_be_bytes([data[36], data[37]]);
    SppParseResult::Found {
        header: SppHeader {
            client_addr,
            client_port,
            proxy_addr,
            proxy_port,
        },
        payload_offset: SPP_HEADER_SIZE,
    }
}

/// Returns `true` when the datagram starts with the Simple Proxy Protocol magic bytes.
#[inline]
pub fn has_spp_magic(data: &[u8]) -> bool {
    data.len() >= 2 && data[0] == 0x56 && data[1] == 0xEC
}