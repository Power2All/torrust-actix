use crate::config::structs::proxy_config::ProxyConfig;
use crate::tracker::structs::announce_response::AnnounceResponse;
use crate::tracker::structs::rtc_answer::RtcAnswer;
use bip_bencode::{
    BDecodeOpt,
    BRefAccess,
    BencodeRef
};

pub fn build_reqwest_client(proxy: Option<&ProxyConfig>) -> reqwest::Client {
    let mut builder = reqwest::Client::builder().timeout(std::time::Duration::from_secs(15));
    if let Some(p) = proxy {
        let scheme = match p.proxy_type.as_str() {
            "http" | "http_auth" => "http",
            "socks4" => "socks4",
            "socks5" | "socks5_auth" => "socks5",
            other => {
                log::warn!("[Proxy] Unknown proxy type '{}' — ignoring proxy", other);
                return builder.build().expect("failed to build reqwest client");
            }
        };
        let proxy_url = format!("{}://{}:{}", scheme, p.host, p.port);
        match reqwest::Proxy::all(&proxy_url) {
            Ok(mut proxy_cfg) => {
                if let (Some(user), Some(pass)) = (&p.username, &p.password) {
                    proxy_cfg = proxy_cfg.basic_auth(user, pass);
                }
                builder = builder.proxy(proxy_cfg);
            }
            Err(e) => {
                log::warn!("[Proxy] Failed to build proxy '{}': {} — ignoring", proxy_url, e);
            }
        }
    }
    builder.build().expect("failed to build reqwest client")
}

pub fn parse_announce_response(body: &[u8]) -> AnnounceResponse {
    let mut out = AnnounceResponse::default();
    let decoded = match BencodeRef::decode(body, BDecodeOpt::default()) {
        Ok(b) => b,
        Err(e) => {
            log::warn!("[Tracker] Bencode parse error: {e}");
            return out;
        }
    };
    let dict = match decoded.dict() {
        Some(d) => d,
        None => {
            log::warn!("[Tracker] Response is not a bencode dict");
            return out;
        }
    };
    if let Some(reason) = dict.lookup(b"failure reason") {
        if let Some(bytes) = reason.bytes() {
            out.failure_reason = Some(String::from_utf8_lossy(bytes).to_string());
            log::error!("[Tracker] Failure: {:?}", out.failure_reason);
        }
        return out;
    }
    if let Some(v) = dict.lookup(b"interval")
        && let Some(n) = v.int() {
            out.interval = n as u64;
        }
    if let Some(v) = dict.lookup(b"rtc interval")
        && let Some(n) = v.int() {
            out.rtc_interval = Some(n as u64);
        }
    if let Some(answers) = dict.lookup(b"rtc_answers")
        && let Some(list) = answers.list() {
            for item in list {
                if let Some(d) = item.dict() {
                    let peer_id_hex = d
                        .lookup(b"peer_id")
                        .and_then(|v| v.bytes())
                        .map(hex::encode)
                        .unwrap_or_default();
                    let sdp_answer = d
                        .lookup(b"sdp_answer")
                        .and_then(|v| v.bytes())
                        .map(|b| String::from_utf8_lossy(b).to_string())
                        .unwrap_or_default();
                    if !peer_id_hex.is_empty() && !sdp_answer.is_empty() {
                        out.rtc_answers.push(RtcAnswer { peer_id_hex, sdp_answer });
                    }
                }
            }
        }
    out
}