use crate::common::structs::custom_error::CustomError;
use rand::RngExt;

pub const MAX_PERCENT_DECODED_SIZE: usize = 1_048_576;
pub const MAX_WEBRTC_SDP_SIZE: usize = 262_144;
pub const MIN_API_KEY_LENGTH: usize = 32;
pub const DEFAULT_API_KEY_ENTROPY_BYTES: usize = 32;
pub const MAX_INFO_HASH_HEX_LENGTH: usize = 40;
pub const MAX_PEER_ID_HEX_LENGTH: usize = 40;
pub const MAX_SCRAPE_TORRENTS: usize = 100;
pub const MAX_OFFER_ID_LENGTH: usize = 128;
pub const MAX_QUERY_STRING_LENGTH: usize = 8192;

pub fn generate_secure_api_key() -> String {
    let mut rng = rand::rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.random()).collect();
    use base64::prelude::*;
    BASE64_URL_SAFE_NO_PAD.encode(&bytes)
}

pub fn validate_api_key_strength(api_key: &str) -> bool {
    if api_key.len() < MIN_API_KEY_LENGTH {
        return false;
    }
    let has_lower = api_key.chars().any(|c| c.is_ascii_lowercase());
    let has_upper = api_key.chars().any(|c| c.is_ascii_uppercase());
    let has_digit = api_key.chars().any(|c| c.is_ascii_digit());
    let has_special = api_key.chars().any(|c| !c.is_alphanumeric());
    let variety_count = [has_lower, has_upper, has_digit, has_special]
        .iter()
        .filter(|&&x| x)
        .count();
    variety_count >= 2
}

pub fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();
    let mut result = 0u8;
    for (x, y) in a_bytes.iter().zip(b_bytes.iter()) {
        result |= x ^ y;
    }
    result == 0
}

pub fn validate_file_path(path: &str) -> Result<(), CustomError> {
    if path.contains("..") || path.contains("./") || path.contains(".\\") {
        return Err(CustomError::new("Path traversal detected in file path"));
    }
    if path.starts_with('/') || (path.len() > 2 && path[1..].starts_with(":\\")) {
        return Err(CustomError::new("Absolute paths not allowed in certificate configuration"));
    }
    if path.contains('\0') {
        return Err(CustomError::new("Null byte detected in file path"));
    }
    Ok(())
}

pub fn validate_webrtc_sdp(sdp: &str) -> Result<(), CustomError> {
    if sdp.len() > MAX_WEBRTC_SDP_SIZE {
        return Err(CustomError::new(&format!(
            "WebRTC SDP exceeds maximum size of {} bytes",
            MAX_WEBRTC_SDP_SIZE
        )));
    }
    if !sdp.starts_with("v=") && !sdp.starts_with('{') && !sdp.starts_with('"') {
        return Err(CustomError::new("Invalid WebRTC SDP format: must start with v= or be JSON"));
    }
    let suspicious_patterns = ["<script", "javascript:", "data:", "vbscript:"];
    let sdp_lower = sdp.to_lowercase();
    for pattern in suspicious_patterns {
        if sdp_lower.contains(pattern) {
            return Err(CustomError::new("Suspicious content detected in WebRTC SDP"));
        }
    }
    Ok(())
}

pub fn validate_info_hash_hex(info_hash: &str) -> Result<(), CustomError> {
    if info_hash.len() > MAX_INFO_HASH_HEX_LENGTH {
        return Err(CustomError::new("info_hash exceeds maximum length"));
    }
    if !info_hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(CustomError::new("info_hash contains invalid hex characters"));
    }
    Ok(())
}

pub fn validate_peer_id_hex(peer_id: &str) -> Result<(), CustomError> {
    if peer_id.len() > MAX_PEER_ID_HEX_LENGTH {
        return Err(CustomError::new("peer_id exceeds maximum length"));
    }
    if !peer_id.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(CustomError::new("peer_id contains invalid hex characters"));
    }
    Ok(())
}

pub fn validate_query_string_length(query: &str) -> Result<(), CustomError> {
    if query.len() > MAX_QUERY_STRING_LENGTH {
        return Err(CustomError::new(&format!(
            "Query string exceeds maximum length of {} bytes",
            MAX_QUERY_STRING_LENGTH
        )));
    }
    Ok(())
}

pub fn validate_remote_ip(ip: &str, trusted_proxies_enabled: bool) -> Result<(), CustomError> {
    use std::net::IpAddr;

    let addr: IpAddr = ip.parse().map_err(|_| CustomError::new("Invalid IP address format"))?;
    if !trusted_proxies_enabled {
        let is_private = match addr {
            IpAddr::V4(ipv4) => {
                ipv4.is_loopback() || ipv4.is_private() || ipv4.is_link_local() || ipv4.is_unspecified()
            }
            IpAddr::V6(ipv6) => {
                ipv6.is_loopback() || ipv6.is_unspecified()
            }
        };
        if is_private {
            return Err(CustomError::new(
                "Private IP addresses not allowed in X-Real-IP header without trusted proxy configuration"
            ));
        }
    }
    Ok(())
}