use crate::seeder::types::{
    CHUNK_SIZE,
    MSG_PIECE_CHUNK,
    MSG_PIECE_DATA,
    MSG_PIECE_REQUEST,
    SCTP_MAX_PAYLOAD
};
use crate::torrent::structs::torrent_info::TorrentInfo;
use bytes::Bytes;
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
use std::num::NonZeroU32;
use std::sync::atomic::{
    AtomicU64,
    Ordering
};
use std::sync::Arc;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::data_channel::RTCDataChannel;

pub type SharedRateLimiter = Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>;

pub fn generate_peer_id() -> [u8; 20] {
    let prefix = b"-RS1000-";
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

pub fn setup_handlers(
    dc: Arc<RTCDataChannel>,
    torrent_info: Arc<TorrentInfo>,
    uploaded: Arc<AtomicU64>,
    rate_limiter: Option<SharedRateLimiter>,
) {
    let dc_msg = Arc::clone(&dc);
    let ti_msg = Arc::clone(&torrent_info);
    let up_msg = Arc::clone(&uploaded);
    let rl_msg = rate_limiter;
    dc.on_open(Box::new(|| {
        log::info!("[Seeder] Data channel opened");
        Box::pin(async {})
    }));
    dc.on_close(Box::new(|| {
        log::info!("[Seeder] Data channel closed");
        Box::pin(async {})
    }));
    dc.on_error(Box::new(|e| {
        log::error!("[Seeder] Data channel error: {}", e);
        Box::pin(async {})
    }));
    dc.on_message(Box::new(move |msg: DataChannelMessage| {
        let data = msg.data.clone();
        let dc2 = Arc::clone(&dc_msg);
        let ti2 = Arc::clone(&ti_msg);
        let up2 = Arc::clone(&up_msg);
        let rl2 = rl_msg.clone();
        Box::pin(async move {
            handle_message(data, dc2, ti2, up2, rl2).await;
        })
    }));
}

async fn handle_message(
    data: Bytes,
    dc: Arc<RTCDataChannel>,
    torrent_info: Arc<TorrentInfo>,
    uploaded: Arc<AtomicU64>,
    rate_limiter: Option<SharedRateLimiter>,
) {
    if data.len() < 5 {
        return;
    }
    if data[0] != MSG_PIECE_REQUEST {
        return;
    }
    let piece_index = u32::from_be_bytes([data[1], data[2], data[3], data[4]]) as usize;
    log::debug!("[Seeder] Piece request: {}", piece_index);
    match read_piece(&torrent_info, piece_index) {
        Ok(piece_data) => {
            if let Some(rl) = &rate_limiter {
                let n = NonZeroU32::new(piece_data.len() as u32).unwrap_or(NonZeroU32::MIN);
                rl.until_n_ready(n).await.ok();
            }
            let bytes_sent = send_piece(&dc, piece_index, &piece_data).await;
            match bytes_sent {
                Ok(n) => {
                    uploaded.fetch_add(n, Ordering::Relaxed);
                    log::debug!("[Seeder] Sent piece {} ({} bytes)", piece_index, n);
                }
                Err(e) => {
                    log::error!("[Seeder] Send error for piece {}: {}", piece_index, e);
                }
            }
        }
        Err(e) => {
            log::error!("[Seeder] Failed to read piece {}: {}", piece_index, e);
        }
    }
}

async fn send_piece(
    dc: &RTCDataChannel,
    piece_index: usize,
    piece_data: &[u8],
) -> Result<u64, webrtc::Error> {
    if piece_data.len() <= SCTP_MAX_PAYLOAD {
        let mut frame = vec![0u8; 5 + piece_data.len()];
        frame[0] = MSG_PIECE_DATA;
        frame[1..5].copy_from_slice(&(piece_index as u32).to_be_bytes());
        frame[5..].copy_from_slice(piece_data);
        dc.send(&Bytes::from(frame)).await?;
    } else {
        let total_size = piece_data.len() as u32;
        let mut offset = 0usize;
        while offset < piece_data.len() {
            let end = (offset + CHUNK_SIZE).min(piece_data.len());
            let chunk = &piece_data[offset..end];
            let mut frame = vec![0u8; 13 + chunk.len()];
            frame[0] = MSG_PIECE_CHUNK;
            frame[1..5].copy_from_slice(&(piece_index as u32).to_be_bytes());
            frame[5..9].copy_from_slice(&total_size.to_be_bytes());
            frame[9..13].copy_from_slice(&(offset as u32).to_be_bytes());
            frame[13..].copy_from_slice(chunk);
            dc.send(&Bytes::from(frame)).await?;
            offset = end;
        }
    }
    Ok(piece_data.len() as u64)
}

fn read_piece(info: &TorrentInfo, piece_index: usize) -> std::io::Result<Vec<u8>> {
    let start = piece_index as u64 * info.piece_length;
    let end = (start + info.piece_length).min(info.total_size);
    if start >= info.total_size {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "piece {} out of range (total {} pieces)",
                piece_index, info.piece_count
            ),
        ));
    }
    let need = (end - start) as usize;
    let mut buf = vec![0u8; need];
    let mut filled = 0usize;
    for file_entry in &info.files {
        let file_end = file_entry.offset + file_entry.length;
        let overlap_start = start.max(file_entry.offset);
        let overlap_end = end.min(file_end);
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