use crate::tracker::structs::announce_response::AnnounceResponse;
use bip_bencode::{
    BDecodeOpt,
    BRefAccess,
    BencodeRef
};
use std::net::Ipv4Addr;

pub fn parse_http_announce_response(body: &[u8]) -> AnnounceResponse {
    let mut out = AnnounceResponse::default();
    let decoded = match BencodeRef::decode(body, BDecodeOpt::default()) {
        Ok(b) => b,
        Err(e) => {
            log::warn!("[Tracker/HTTP] Bencode parse error: {e}");
            return out;
        }
    };
    let dict = match decoded.dict() {
        Some(d) => d,
        None => {
            log::warn!("[Tracker/HTTP] Response is not a bencode dict");
            return out;
        }
    };
    if let Some(reason) = dict.lookup(b"failure reason") {
        if let Some(bytes) = reason.bytes() {
            out.failure_reason = Some(String::from_utf8_lossy(bytes).to_string());
            log::error!("[Tracker/HTTP] Failure: {:?}", out.failure_reason);
        }
        return out;
    }
    if let Some(v) = dict.lookup(b"interval") {
        if let Some(n) = v.int() {
            out.interval = n as u64;
        }
    }
    if let Some(v) = dict.lookup(b"peers") {
        if let Some(bytes) = v.bytes() {
            let mut i = 0;
            while i + 6 <= bytes.len() {
                let ip = Ipv4Addr::new(bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]);
                let port = u16::from_be_bytes([bytes[i + 4], bytes[i + 5]]);
                out.peers.push((ip, port));
                i += 6;
            }
        }
    }
    out
}

pub fn parse_udp_connect_response(buf: &[u8], expected_txid: u32) -> Option<u64> {
    if buf.len() < 16 {
        return None;
    }
    let action = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
    let txid = u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]);
    if action != 0 || txid != expected_txid {
        return None;
    }
    let connection_id = u64::from_be_bytes([
        buf[8], buf[9], buf[10], buf[11],
        buf[12], buf[13], buf[14], buf[15],
    ]);
    Some(connection_id)
}

pub fn parse_udp_announce_response(buf: &[u8], expected_txid: u32) -> Option<AnnounceResponse> {
    if buf.len() < 20 {
        return None;
    }
    let action = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
    let txid = u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]);
    if action != 1 || txid != expected_txid {
        return None;
    }
    let interval = u32::from_be_bytes([buf[8], buf[9], buf[10], buf[11]]) as u64;
    let mut peers = Vec::new();
    let mut i = 20;
    while i + 6 <= buf.len() {
        let ip = Ipv4Addr::new(buf[i], buf[i + 1], buf[i + 2], buf[i + 3]);
        let port = u16::from_be_bytes([buf[i + 4], buf[i + 5]]);
        peers.push((ip, port));
        i += 6;
    }
    Some(AnnounceResponse { interval, peers, failure_reason: None })
}

pub fn parse_udp_tracker_addr(url: &str) -> Option<String> {
    let without_scheme = url.strip_prefix("udp://")?;
    let host_port = without_scheme.split('/').next()?;
    Some(host_port.to_string())
}