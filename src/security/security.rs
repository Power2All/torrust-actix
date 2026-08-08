use crate::common::structs::custom_error::CustomError;
use rand::RngExt;

pub const MAX_PERCENT_DECODED_SIZE: usize = 1_048_576;
pub const MAX_PEER_MESSAGE_SIZE: usize = 262_144;
pub const MIN_API_KEY_LENGTH: usize = 32;
pub const DEFAULT_API_KEY_ENTROPY_BYTES: usize = 32;
pub const MAX_INFO_HASH_HEX_LENGTH: usize = 40;
pub const MAX_PEER_ID_HEX_LENGTH: usize = 40;
pub const MAX_SCRAPE_TORRENTS: usize = 100;
pub const MAX_OFFER_ID_LENGTH: usize = 128;
pub const MAX_QUERY_STRING_LENGTH: usize = 8192;

/// Generates an API key from 32 cryptographically secure random bytes, encoded as a
/// 43-character URL-safe Base64 string (no padding).
pub fn generate_secure_api_key() -> String {
    let mut rng = rand::rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.random()).collect();
    use base64::prelude::*;
    BASE64_URL_SAFE_NO_PAD.encode(&bytes)
}

/// Checks that an API key is at least 32 characters and contains at least two character
/// classes (lowercase, uppercase, digits, specials).
///
/// Used at startup to warn about weak keys; does not reject them.
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

/// Compares two strings in constant time to prevent timing attacks on token checks.
///
/// A length mismatch is folded into the result rather than short-circuiting, so an early return
/// cannot reveal the expected length.
pub fn constant_time_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut result = u8::from(a.len() != b.len());
    for i in 0..a.len().max(b.len()) {
        result |= a.get(i).copied().unwrap_or(0) ^ b.get(i).copied().unwrap_or(0);
    }
    result == 0
}

/// Rejects file paths containing traversal sequences or unexpected characters.
///
/// Absolute paths are allowed: these values come from the configuration file, not from a
/// request, and `/etc/ssl/...` is where a certificate normally lives. Refusing them only forced
/// operators into relative paths without closing anything, since nothing reachable from the
/// network chooses this string.
///
/// # Errors
///
/// Returns a [`CustomError`] describing the violation.
pub fn validate_file_path(path: &str) -> Result<(), CustomError> {
    if path.contains("..") {
        return Err(CustomError::new("Path traversal detected in file path"));
    }
    if path.contains('\0') {
        return Err(CustomError::new("Null byte detected in file path"));
    }
    Ok(())
}

/// Bounds a peer-supplied message (such as an SDP payload) to `MAX_PEER_MESSAGE_SIZE`.
///
/// # Errors
///
/// Returns a [`CustomError`] when the message is too large.
pub fn validate_peer_message(message: &str) -> Result<(), CustomError> {
    if message.len() > MAX_PEER_MESSAGE_SIZE {
        return Err(CustomError::new(&format!(
            "Peer message exceeds maximum size of {MAX_PEER_MESSAGE_SIZE} bytes"
        )));
    }
    Ok(())
}

/// Validates that a string is a 40-character hex info-hash.
///
/// # Errors
///
/// Returns a [`CustomError`] when the format is invalid.
pub fn validate_info_hash_hex(info_hash: &str) -> Result<(), CustomError> {
    if info_hash.len() == MAX_INFO_HASH_HEX_LENGTH && info_hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Ok(());
    }
    Err(CustomError::new("info_hash has invalid format"))
}

/// Validates that a string is a 40-character hex peer id.
///
/// # Errors
///
/// Returns a [`CustomError`] when the format is invalid.
pub fn validate_peer_id_hex(peer_id: &str) -> Result<(), CustomError> {
    if peer_id.len() == MAX_PEER_ID_HEX_LENGTH && peer_id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Ok(());
    }
    Err(CustomError::new("peer_id has invalid format"))
}

/// Bounds a raw query string to `MAX_QUERY_STRING_LENGTH`.
///
/// # Errors
///
/// Returns a [`CustomError`] when the query is too long.
pub fn validate_query_string_length(query: &str) -> Result<(), CustomError> {
    if query.len() > MAX_QUERY_STRING_LENGTH {
        return Err(CustomError::new(&format!(
            "Query string exceeds maximum length of {MAX_QUERY_STRING_LENGTH} bytes"
        )));
    }
    Ok(())
}

/// Validates a client-supplied IP string (from a proxy header) before parsing it.
///
/// With `trusted_proxies_enabled = false`, loopback, unspecified and private values are
/// rejected (IPv4 private/link-local, IPv6 `fc00::/7` and `fe80::/10`) so an untrusted sender
/// cannot claim an internal address.
///
/// # Errors
///
/// Returns a [`CustomError`] when the value is not a plausible IP or proxies are not trusted.
pub fn validate_remote_ip(ip: &str, trusted_proxies_enabled: bool) -> Result<(), CustomError> {
    use std::net::IpAddr;

    let addr: IpAddr = ip.parse().map_err(|_| CustomError::new("Invalid IP address format"))?;
    if !trusted_proxies_enabled {
        let is_private = match addr {
            IpAddr::V4(ipv4) => {
                ipv4.is_loopback() || ipv4.is_private() || ipv4.is_link_local() || ipv4.is_unspecified()
            }
            IpAddr::V6(ipv6) => {
                ipv6.is_loopback()
                    || ipv6.is_unspecified()
                    // fc00::/7 and fe80::/10, the IPv6 counterparts of the private and
                    // link-local ranges rejected above for IPv4.
                    || ipv6.is_unique_local()
                    || ipv6.is_unicast_link_local()
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