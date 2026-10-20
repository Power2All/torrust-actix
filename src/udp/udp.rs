use crate::config::structs::udp_trackers_config::UdpTrackersConfig;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::udp::enums::simple_proxy_protocol::SppParseResult;
use crate::udp::structs::simple_proxy_protocol::SppHeader;
use crate::udp::structs::udp_server::UdpServer;
use log::{
    error,
    info
};
use std::io::{
    Error,
    Read
};
use std::net::{
    IpAddr,
    Ipv4Addr,
    Ipv6Addr
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

/// Spawns the UDP tracker service described by `config` using its receive backend and returns
/// its join handle. The service runs until the shutdown watch channel fires.
pub async fn udp_service(config: &UdpTrackersConfig, data: Arc<TorrentTracker>, rx: tokio::sync::watch::Receiver<bool>, tokio_udp: Arc<Runtime>) -> JoinHandle<()>
{
    let udp_server = UdpServer::new(data, config).await.unwrap_or_else(|e| {
        error!("Could not listen to the UDP port: {e}");
        exit(1);
    });
    let spp_status = if config.simple_proxy_protocol { " with Simple Proxy Protocol enabled" } else { "" };
    info!("[UDP] Starting a server listener on {} with {} UDP threads and {} worker threads{spp_status}", config.bind_address, config.udp_threads, config.worker_threads);
    tokio_udp.spawn(async move {
        udp_server.start(rx).await;
    })
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