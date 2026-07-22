use crate::common::structs::compressed_bytes::COMPRESSION;
use crate::common::structs::compression_state::CompressionState;
use crate::common::structs::custom_error::CustomError;
use crate::common::types::QueryValues;
use crate::config::enums::compression_algorithm::CompressionAlgorithm;
use crate::config::structs::configuration::Configuration;
use crate::security::security::MAX_PERCENT_DECODED_SIZE;
use fern::colors::{
    Color,
    ColoredLevelConfig
};
use log::info;
use sha1::Digest;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;
use std::time::{
    Duration,
    SystemTime
};
use tokio_shutdown::Shutdown;

/// Parses a URL query string into a map of lower-cased keys to raw (percent-decoded) values.
///
/// Values are kept as bytes because BitTorrent info-hashes and peer ids are binary.
/// Repeated keys accumulate multiple values.
///
/// # Errors
///
/// Returns a [`CustomError`] when a decoded value exceeds `MAX_PERCENT_DECODED_SIZE`.
#[inline]
pub fn parse_query(query: Option<&str>) -> Result<HashMap<String, QueryValues>, CustomError> {
    let mut queries: HashMap<String, QueryValues> = HashMap::with_capacity(12);
    if let Some(result) = query {
        for query_item in result.split('&') {
            if query_item.is_empty() {
                continue;
            }
            if let Some(equal_pos) = query_item.find('=') {
                let (key_part, value_part) = query_item.split_at(equal_pos);
                let key_name_raw = key_part;
                let value_data_raw = &value_part[1..];
                let key_name = if key_name_raw.contains('%') || key_name_raw.contains('+') {
                    percent_encoding::percent_decode_str(key_name_raw)
                        .decode_utf8_lossy()
                        .to_lowercase()
                } else {
                    key_name_raw.to_ascii_lowercase()
                };
                if key_name.is_empty() {
                    continue;
                }
                let value_data = percent_encoding::percent_decode_str(value_data_raw).collect::<Vec<u8>>();
                if value_data.len() > MAX_PERCENT_DECODED_SIZE {
                    return Err(CustomError::new(&format!(
                        "Percent-decoded value exceeds maximum size of {MAX_PERCENT_DECODED_SIZE} bytes"
                    )));
                }
                queries
                    .entry(key_name)
                    .or_default()
                    .push(value_data);
            } else {
                let key_name = if query_item.contains('%') || query_item.contains('+') {
                    percent_encoding::percent_decode_str(query_item)
                        .decode_utf8_lossy()
                        .to_lowercase()
                } else {
                    query_item.to_ascii_lowercase()
                };
                if key_name.is_empty() {
                    continue;
                }
                queries
                    .entry(key_name)
                    .or_default()
                    .push(Vec::new());
            }
        }
    }
    Ok(queries)
}

/// Windows-only sanity check that a UDP bind address is available before spawning the server.
///
/// # Errors
///
/// Returns the bind error when the address is already in use.
pub fn udp_check_host_and_port_used(bind_address: &str) -> std::io::Result<()> {
    if cfg!(target_os = "windows")
        && let Err(e) = std::net::UdpSocket::bind(bind_address)
    {
        sentry::capture_error(&e);
        log::error!("Unable to bind UDP socket to {bind_address}: {e}");
        return Err(e);
    }
    Ok(())
}

/// Error returned when parsing a 40-character hex string into a 20-byte id fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum HexParseError {
    #[error("invalid input length")]
    InvalidLength,
    #[error("invalid hex character")]
    InvalidCharacter,
}

/// Formats a 20-byte binary hash as 40 lowercase hex characters into `f`.
pub(crate) fn bin2hex(data: &[u8; 20], f: &mut Formatter) -> fmt::Result {
    let mut chars = [0u8; 40];
    hex::encode_to_slice(data, &mut chars).expect("failed to hexlify");
    write!(f, "{}", std::str::from_utf8(&chars).unwrap())
}

pub struct Hex20(pub [u8; 40]);

impl Hex20 {
    /// Returns the hex string as `&str` (always 40 valid ASCII characters).
    #[inline]
    pub fn as_str(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(&self.0) }
    }
}

/// Converts a 20-byte binary hash into a stack-allocated 40-character lowercase hex buffer.
#[inline]
pub fn bin20_to_hex(data: &[u8; 20]) -> Hex20 {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
    let mut buffer = [0u8; 40];
    for (i, &byte) in data.iter().enumerate() {
        let idx = i * 2;
        buffer[idx] = HEX_CHARS[(byte >> 4) as usize];
        buffer[idx + 1] = HEX_CHARS[(byte & 0xf) as usize];
    }
    Hex20(buffer)
}

/// Decodes a 40-character hex string into a 20-byte binary hash.
///
/// # Errors
///
/// Returns a [`CustomError`] when the input is not valid hex or is too short.
pub fn hex2bin(data: String) -> Result<[u8; 20], CustomError> {
    hex::decode(data)
        .map_err(|data| {
            sentry::capture_error(&data);
            CustomError::new("error converting hex to bin")
        })
        .and_then(|hash_result| {
            hash_result
                .get(..20)
                .and_then(|slice| slice.try_into().ok())
                .ok_or_else(|| CustomError::new("invalid hex length"))
        })
}

/// Initialises the global `fern` logger with coloured output at the configured log level.
///
/// # Panics
///
/// Panics on an unknown `log_level` value or when a logger is already installed.
pub fn setup_logging(config: &Configuration) {
    let level = match config.log_level.as_str() {
        "off" => log::LevelFilter::Off,
        "trace" => log::LevelFilter::Trace,
        "debug" => log::LevelFilter::Debug,
        "info" => log::LevelFilter::Info,
        "warn" => log::LevelFilter::Warn,
        "error" => log::LevelFilter::Error,
        _ => {
            panic!("Unknown log level encountered: '{}'", config.log_level.as_str());
        }
    };
    let colors = ColoredLevelConfig::new()
        .trace(Color::Cyan)
        .debug(Color::Magenta)
        .info(Color::Green)
        .warn(Color::Yellow)
        .error(Color::Red);
    fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{} [{:width$}][{}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.9f"),
                colors.color(record.level()),
                record.target(),
                message,
                width = 5
            ));
        })
        .level(level)
        .chain(std::io::stdout())
        .apply()
        .unwrap_or_else(|_| panic!("Failed to initialize logging."));
    info!("logging initialized.");
}

/// Returns the current Unix timestamp in seconds.
#[inline]
pub fn current_time() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("System time before UNIX epoch")
        .as_secs()
}

/// Encodes an integer as big-endian bytes with leading zero bytes stripped.
#[inline]
pub fn convert_int_to_bytes(number: &u64) -> Vec<u8> {
    let bytes = number.to_be_bytes();
    let leading_zeros = number.leading_zeros() as usize / 8;
    bytes[leading_zeros..].to_vec()
}

/// Decodes up to 8 big-endian bytes (as produced by [`convert_int_to_bytes`]) into a `u64`.
#[inline]
pub fn convert_bytes_to_int(array: &[u8]) -> u64 {
    let mut array_fixed = [0u8; 8];
    let len = array.len().min(8);
    array_fixed[8 - len..].copy_from_slice(&array[..len]);
    u64::from_be_bytes(array_fixed)
}

/// Waits up to `timeout` for the shutdown signal.
///
/// Returns `true` when shutdown was signalled, `false` when the timeout elapsed.
pub async fn shutdown_waiting(timeout: Duration, shutdown_handler: Shutdown) -> bool {
    tokio::time::timeout(timeout, shutdown_handler.handle())
        .await
        .is_ok()
}

/// Returns the SHA-1 digest of the given string, used to derive stable 20-byte identifiers.
pub fn hash_id(id: &str) -> [u8; 20] {
    let mut hasher = sha1::Sha1::new();
    hasher.update(id.as_bytes());
    <[u8; 20]>::try_from(hasher.finalize().as_slice()).unwrap()
}

/// Converts one ASCII hex character to its 4-bit value, or `0xFF` for invalid input.
#[inline(always)]
pub fn hex_to_nibble(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => 0xFF,
    }
}

/// Sets the global in-memory compression settings (algorithm and level) used for SDP storage.
///
/// Only the first call takes effect.
pub fn init_compression(enabled: bool, algorithm: CompressionAlgorithm, level: u32) {
    let _ = COMPRESSION.set(CompressionState { enabled, algorithm, level });
}