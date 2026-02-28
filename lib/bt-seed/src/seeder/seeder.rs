use crate::seeder::structs::peer_count_guard::PeerCountGuard;
use crate::seeder::types::{
    BT_HANDSHAKE_LEN,
    BT_PROTOCOL,
    MAX_BLOCK_SIZE,
    MSG_BITFIELD,
    MSG_CANCEL,
    MSG_CHOKE,
    MSG_HAVE,
    MSG_INTERESTED,
    MSG_NOT_INTERESTED,
    MSG_PIECE,
    MSG_REQUEST,
    MSG_UNCHOKE
};
use crate::torrent::structs::torrent_info::TorrentInfo;
use governor::clock::DefaultClock;
use governor::state::{
    InMemoryState,
    NotKeyed
};
use governor::RateLimiter;
use rand::RngExt;
use std::fs::File;
use std::io::{
    Read,
    Seek,
    SeekFrom
};
use std::net::SocketAddr;
use std::num::NonZeroU32;
use std::sync::atomic::{
    AtomicU64,
    AtomicUsize,
    Ordering
};
use std::sync::Arc;
use tokio::io::{
    AsyncReadExt,
    AsyncWriteExt
};
use tokio::net::TcpStream;

pub type SharedRateLimiter = Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>;

pub fn generate_peer_id() -> [u8; 20] {
    let prefix = b"-BS1000-";
    let mut id = [0u8; 20];
    id[..prefix.len()].copy_from_slice(prefix);
    rand::rng().fill(&mut id[prefix.len()..]);
    for b in &mut id[prefix.len()..] {
        *b = b'0' + (*b % 10);
    }
    id
}

pub fn fmt_bytes(n: u64) -> String {
    if n < 1024 {
        format!("{} B", n)
    } else if n < 1024 * 1024 {
        format!("{:.1} KB", n as f64 / 1024.0)
    } else if n < 1024 * 1024 * 1024 {
        format!("{:.1} MB", n as f64 / 1024.0 / 1024.0)
    } else {
        format!("{:.2} GB", n as f64 / 1024.0 / 1024.0 / 1024.0)
    }
}

pub fn make_handshake(info_hash: &[u8; 20], peer_id: &[u8; 20]) -> [u8; BT_HANDSHAKE_LEN] {
    let mut hs = [0u8; BT_HANDSHAKE_LEN];
    hs[0..20].copy_from_slice(BT_PROTOCOL);
    hs[28..48].copy_from_slice(info_hash);
    hs[48..68].copy_from_slice(peer_id);
    hs
}

pub fn read_block(info: &TorrentInfo, piece_index: usize, begin: u64, length: usize) -> std::io::Result<Vec<u8>> {
    let piece_start = piece_index as u64 * info.piece_length;
    let block_start = piece_start + begin;
    let block_end = (block_start + length as u64).min(info.total_size);
    if block_start >= info.total_size {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("block out of range: piece={} begin={}", piece_index, begin),
        ));
    }
    let actual_len = (block_end - block_start) as usize;
    let mut buf = vec![0u8; actual_len];
    let mut filled = 0usize;
    for file_entry in &info.files {
        let file_end = file_entry.offset + file_entry.length;
        let overlap_start = block_start.max(file_entry.offset);
        let overlap_end = block_end.min(file_end);
        if overlap_start >= overlap_end {
            continue;
        }
        let in_file_start = overlap_start - file_entry.offset;
        let n = (overlap_end - overlap_start) as usize;
        let mut f = File::open(&file_entry.path)?;
        f.seek(SeekFrom::Start(in_file_start))?;
        f.read_exact(&mut buf[filled..filled + n])?;
        filled += n;
    }
    Ok(buf)
}

async fn send_bitfield(stream: &mut TcpStream, piece_count: usize) -> std::io::Result<()> {
    let bitfield_len = piece_count.div_ceil(8);
    let mut bitfield = vec![0xFFu8; bitfield_len];
    let extra_bits = bitfield_len * 8 - piece_count;
    if extra_bits > 0
        && let Some(last) = bitfield.last_mut() {
            *last &= 0xFF << extra_bits;
        }
    let msg_len = 1 + bitfield_len;
    let mut msg = Vec::with_capacity(4 + msg_len);
    msg.extend_from_slice(&(msg_len as u32).to_be_bytes());
    msg.push(MSG_BITFIELD);
    msg.extend_from_slice(&bitfield);
    stream.write_all(&msg).await
}

async fn send_unchoke(stream: &mut TcpStream) -> std::io::Result<()> {
    let msg: [u8; 5] = [0, 0, 0, 1, MSG_UNCHOKE];
    stream.write_all(&msg).await
}

async fn send_piece_block(
    stream: &mut TcpStream,
    index: u32,
    begin: u32,
    data: &[u8],
) -> std::io::Result<()> {
    let msg_len = 1 + 4 + 4 + data.len();
    let mut msg = Vec::with_capacity(4 + msg_len);
    msg.extend_from_slice(&(msg_len as u32).to_be_bytes());
    msg.push(MSG_PIECE);
    msg.extend_from_slice(&index.to_be_bytes());
    msg.extend_from_slice(&begin.to_be_bytes());
    msg.extend_from_slice(data);
    stream.write_all(&msg).await
}

async fn read_message(stream: &mut TcpStream) -> std::io::Result<Option<(u8, Vec<u8>)>> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let msg_len = u32::from_be_bytes(len_buf) as usize;
    if msg_len == 0 {
        return Ok(None);
    }
    let mut msg_buf = vec![0u8; msg_len];
    stream.read_exact(&mut msg_buf).await?;
    let id = msg_buf[0];
    let payload = msg_buf[1..].to_vec();
    Ok(Some((id, payload)))
}

pub async fn handle_peer(
    mut stream: TcpStream,
    addr: SocketAddr,
    info_hash: [u8; 20],
    our_peer_id: [u8; 20],
    torrent_info: Arc<TorrentInfo>,
    uploaded: Arc<AtomicU64>,
    peer_count: Arc<AtomicUsize>,
    rate_limiter: Option<SharedRateLimiter>,
) {
    let mut hs_buf = [0u8; BT_HANDSHAKE_LEN];
    if let Err(e) = stream.read_exact(&mut hs_buf).await {
        log::debug!("[BT] Handshake read failed from {}: {}", addr, e);
        return;
    }
    if &hs_buf[0..20] != BT_PROTOCOL {
        log::debug!("[BT] Invalid protocol header from {}", addr);
        return;
    }
    if hs_buf[28..48] != info_hash {
        log::debug!("[BT] Info hash mismatch from {}", addr);
        return;
    }
    let peer_id_hex = hex::encode(&hs_buf[48..68]);
    let our_hs = make_handshake(&info_hash, &our_peer_id);
    if let Err(e) = stream.write_all(&our_hs).await {
        log::debug!("[BT] Handshake write failed to {}: {}", addr, e);
        return;
    }
    peer_count.fetch_add(1, Ordering::Relaxed);
    let _guard = PeerCountGuard { count: Arc::clone(&peer_count) };
    log::info!(
        "[BT] Peer connected: {} ({}…)",
        addr,
        peer_id_hex.get(..8).unwrap_or(&peer_id_hex)
    );
    if let Err(e) = send_bitfield(&mut stream, torrent_info.piece_count).await {
        log::debug!("[BT] Bitfield send failed to {}: {}", addr, e);
        return;
    }
    if let Err(e) = send_unchoke(&mut stream).await {
        log::debug!("[BT] Unchoke send failed to {}: {}", addr, e);
        return;
    }
    loop {
        match read_message(&mut stream).await {
            Ok(None) => {}
            Ok(Some((id, payload))) => {
                match id {
                    MSG_INTERESTED => {
                        log::debug!(
                            "[BT] Peer {}… interested",
                            peer_id_hex.get(..8).unwrap_or(&peer_id_hex)
                        );
                    }
                    MSG_NOT_INTERESTED => {
                        log::debug!(
                            "[BT] Peer {}… not interested",
                            peer_id_hex.get(..8).unwrap_or(&peer_id_hex)
                        );
                    }
                    MSG_REQUEST => {
                        if payload.len() < 12 {
                            log::warn!("[BT] Malformed request from {}", addr);
                            break;
                        }
                        let index = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
                        let begin = u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]);
                        let length = u32::from_be_bytes([payload[8], payload[9], payload[10], payload[11]]);
                        let length = length.min(MAX_BLOCK_SIZE);
                        log::debug!("[BT] Request: piece={} begin={} len={}", index, begin, length);
                        match read_block(&torrent_info, index as usize, begin as u64, length as usize) {
                            Ok(data) => {
                                if let Some(rl) = &rate_limiter {
                                    let n = NonZeroU32::new(data.len() as u32)
                                        .unwrap_or(NonZeroU32::MIN);
                                    rl.until_n_ready(n).await.ok();
                                }
                                let bytes_sent = data.len() as u64;
                                if let Err(e) = send_piece_block(&mut stream, index, begin, &data).await {
                                    log::debug!("[BT] Send error to {}: {}", addr, e);
                                    break;
                                }
                                uploaded.fetch_add(bytes_sent, Ordering::Relaxed);
                            }
                            Err(e) => {
                                log::warn!("[BT] Block read error: {}", e);
                                break;
                            }
                        }
                    }
                    MSG_CANCEL | MSG_CHOKE | MSG_UNCHOKE | MSG_HAVE => {}
                    _ => {
                        log::debug!("[BT] Unknown message id={} from {}", id, addr);
                    }
                }
            }
            Err(e) => {
                log::debug!("[BT] Connection closed from {}: {}", addr, e);
                break;
            }
        }
    }

    log::info!(
        "[BT] Peer disconnected: {} ({}…)",
        addr,
        peer_id_hex.get(..8).unwrap_or(&peer_id_hex)
    );
}