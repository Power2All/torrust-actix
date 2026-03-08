use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::udp::enums::simple_proxy_protocol::SppParseResult;
use crate::udp::structs::number_of_downloads::NumberOfDownloads;
use crate::udp::structs::number_of_peers::NumberOfPeers;
use crate::udp::structs::port::Port;
use crate::udp::structs::response_peer::ResponsePeer;
use crate::udp::structs::simple_proxy_protocol::SppHeader;
use crate::udp::structs::torrent_scrape_statistics::TorrentScrapeStatistics;
use crate::udp::structs::udp_server::UdpServer;
use byteorder::{
    NetworkEndian,
    ReadBytesExt
};
use log::{
    error,
    info
};
use std::io::{
    Cursor,
    Error,
    ErrorKind
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

#[allow(clippy::too_many_arguments)]
pub async fn udp_service(addr: SocketAddr, udp_threads: usize, worker_threads: usize, recv_buffer_size: usize, send_buffer_size: usize, reuse_address: bool, use_payload_ip: bool, simple_proxy_protocol: bool, data: Arc<TorrentTracker>, rx: tokio::sync::watch::Receiver<bool>, tokio_udp: Arc<Runtime>) -> JoinHandle<()>
{
    let udp_server = UdpServer::new(data, addr, udp_threads, worker_threads, recv_buffer_size, send_buffer_size, reuse_address, use_payload_ip, simple_proxy_protocol).await.unwrap_or_else(|e| {
        error!("Could not listen to the UDP port: {e}");
        exit(1);
    });
    let spp_status = if simple_proxy_protocol { " with Simple Proxy Protocol enabled" } else { "" };
    info!("[UDP] Starting a server listener on {addr} with {udp_threads} UDP threads and {worker_threads} worker threads{spp_status}");
    tokio_udp.spawn(async move {
        udp_server.start(rx).await;
    })
}

#[inline]
pub fn parse_ipv4_peers(bytes: &[u8]) -> Result<Vec<ResponsePeer<Ipv4Addr>>, Error> {
    let chunk_size = 6;
    let peer_count = bytes.len() / chunk_size;
    let mut peers = Vec::with_capacity(peer_count);
    for chunk in bytes.chunks_exact(chunk_size) {
        let ip_bytes: [u8; 4] = chunk[..4].try_into().map_err(|_|
            Error::new(ErrorKind::InvalidData, "Invalid IPv4 address bytes")
        )?;
        let port = (&chunk[4..6]).read_u16::<NetworkEndian>().map_err(|e|
            Error::new(ErrorKind::InvalidData, e)
        )?;
        peers.push(ResponsePeer {
            ip_address: Ipv4Addr::from(ip_bytes),
            port: Port(port),
        });
    }
    Ok(peers)
}

#[inline]
pub fn parse_ipv6_peers(bytes: &[u8]) -> Result<Vec<ResponsePeer<Ipv6Addr>>, Error> {
    let chunk_size = 18;
    let peer_count = bytes.len() / chunk_size;
    let mut peers = Vec::with_capacity(peer_count);
    for chunk in bytes.chunks_exact(chunk_size) {
        let ip_bytes: [u8; 16] = chunk[..16].try_into().map_err(|_|
            Error::new(ErrorKind::InvalidData, "Invalid IPv6 address bytes")
        )?;
        let port = (&chunk[16..18]).read_u16::<NetworkEndian>().map_err(|e|
            Error::new(ErrorKind::InvalidData, e)
        )?;
        peers.push(ResponsePeer {
            ip_address: Ipv6Addr::from(ip_bytes),
            port: Port(port),
        });
    }
    Ok(peers)
}

#[inline]
pub fn parse_scrape_stats(bytes: &[u8]) -> Result<Vec<TorrentScrapeStatistics>, Error> {
    let chunk_size = 12;
    let stats_count = bytes.len() / chunk_size;
    let mut stats = Vec::with_capacity(stats_count);
    for chunk in bytes.chunks_exact(chunk_size) {
        let mut cursor = Cursor::new(chunk);
        let seeders = cursor.read_i32::<NetworkEndian>().map_err(|e|
            Error::new(ErrorKind::InvalidData, e)
        )?;
        let downloads = cursor.read_i32::<NetworkEndian>().map_err(|e|
            Error::new(ErrorKind::InvalidData, e)
        )?;
        let leechers = cursor.read_i32::<NetworkEndian>().map_err(|e|
            Error::new(ErrorKind::InvalidData, e)
        )?;
        stats.push(TorrentScrapeStatistics {
            seeders: NumberOfPeers(seeders),
            completed: NumberOfDownloads(downloads),
            leechers: NumberOfPeers(leechers),
        });
    }
    Ok(stats)
}

pub fn parse_address(bytes: &[u8; 16]) -> IpAddr {
    let is_ipv4_mapped = bytes[0..10] == [0u8; 10] && bytes[10] == 0xff && bytes[11] == 0xff;
    if is_ipv4_mapped {
        IpAddr::V4(Ipv4Addr::new(bytes[12], bytes[13], bytes[14], bytes[15]))
    } else {
        IpAddr::V6(Ipv6Addr::from(*bytes))
    }
}

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

#[inline]
pub fn has_spp_magic(data: &[u8]) -> bool {
    data.len() >= 2 && data[0] == 0x56 && data[1] == 0xEC
}