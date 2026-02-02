use crate::common::structs::custom_error::CustomError;
use crate::config::structs::configuration::Configuration;
use async_std::future;
use byteorder::{BigEndian, ReadBytesExt};
use fern::colors::{Color, ColoredLevelConfig};
use log::info;
use smallvec::SmallVec;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;
use std::io::Cursor;
use std::time::{Duration, SystemTime};
use tokio_shutdown::Shutdown;

pub type QueryValues = SmallVec<[Vec<u8>; 1]>;

#[inline]
pub fn parse_query(query: Option<String>) -> Result<HashMap<String, QueryValues>, CustomError> {
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

pub fn udp_check_host_and_port_used(bind_address: String) {
    if cfg!(target_os = "windows") && let Err(data) = std::net::UdpSocket::bind(&bind_address) {
        sentry::capture_error(&data);
        panic!("Unable to bind to {} ! Exiting...", &bind_address);
    }
}

pub(crate) fn bin2hex(data: &[u8; 20], f: &mut Formatter) -> fmt::Result {
    let mut chars = [0u8; 40];
    binascii::bin2hex(data, &mut chars).expect("failed to hexlify");
    write!(f, "{}", std::str::from_utf8(&chars).unwrap())
}

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

pub fn print_type<T>(_: &T) {
    println!("{:?}", std::any::type_name::<T>());
}

pub fn return_type<T>(_: &T) -> String {
    format!("{:?}", std::any::type_name::<T>())
}

pub fn equal_string_check(source: &str, check: &str) -> bool {
    if source == check {
        return true;
    }
    println!("Source: {source}");
    println!("Check:  {check}");
    false
}

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
            ))
        })
        .level(level)
        .chain(std::io::stdout())
        .apply()
        .unwrap_or_else(|_| panic!("Failed to initialize logging."));
    info!("logging initialized.");
}

#[inline]
pub fn current_time() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("System time before UNIX epoch")
        .as_secs()
}

#[inline]
pub fn convert_int_to_bytes(number: &u64) -> Vec<u8> {
    let bytes = number.to_be_bytes();
    let leading_zeros = number.leading_zeros() as usize / 8;
    bytes[leading_zeros..].to_vec()
}

#[inline]
pub fn convert_bytes_to_int(array: &[u8]) -> u64 {
    let mut array_fixed = [0u8; 8];
    let start_idx = 8 - array.len();
    array_fixed[start_idx..].copy_from_slice(array);
    Cursor::new(array_fixed).read_u64::<BigEndian>().unwrap()
}

pub async fn shutdown_waiting(timeout: Duration, shutdown_handler: Shutdown) -> bool {
    future::timeout(timeout, shutdown_handler.handle())
        .await
        .is_ok()
}