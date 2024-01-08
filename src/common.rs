use std::collections::HashMap;
use async_std::sync::Arc;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::fmt::Formatter;
use std::net::{IpAddr, SocketAddr};

use crate::tracker::TorrentTracker;
use crate::udp_common;
use crate::udp_common::AnnounceRequest;

pub fn parse_query(query: Option<String>) -> Result<HashMap<String, Vec<Vec<u8>>>, CustomError> {
    let mut queries: HashMap<String, Vec<Vec<u8>>> = HashMap::new();
    match query {
        None => {}
        Some(result) => {
            let split_raw_query: Vec<&str> = result.split('&').collect();
            for query_item in split_raw_query {
                // Check if the query item actually contains data
                if !query_item.is_empty() {
                    // Check if it's a single key with no data, or key with data
                    if query_item.contains('=') {
                        let key_name_raw = query_item.split('=').collect::<Vec<&str>>()[0];
                        let key_name = percent_encoding::percent_decode_str(key_name_raw).decode_utf8_lossy().to_lowercase();
                        if !key_name.is_empty() {
                            let value_data_raw = query_item.split('=').collect::<Vec<&str>>()[1];
                            let value_data = percent_encoding::percent_decode_str(value_data_raw).collect::<Vec<u8>>();
                            match queries.get(&key_name) {
                                None => {
                                    let query: Vec<Vec<u8>> = vec![value_data];
                                    let _ = queries.insert(key_name, query);
                                }
                                Some(result) => {
                                    let mut result_mut = result.clone();
                                    result_mut.push(value_data);
                                    let _ = queries.insert(key_name, result_mut);
                                }
                            }
                        }
                    } else {
                        let key_name = percent_encoding::percent_decode_str(query_item).decode_utf8_lossy().to_lowercase();
                        if !key_name.is_empty() {
                            match queries.get(&key_name) {
                                None => {
                                    let query: Vec<Vec<u8>> = vec![];
                                    let _ = queries.insert(key_name, query);
                                }
                                Some(result) => {
                                    let mut result_mut = result.clone();
                                    result_mut.push(vec![]);
                                    let _ = queries.insert(key_name, result.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(queries)
}

#[derive(Debug)]
pub struct CustomError {
    pub(crate) message: String,
}

impl CustomError {
    pub fn new(msg: &str) -> CustomError {
        CustomError { message: msg.to_string() }
    }
}

impl fmt::Display for CustomError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for CustomError {
    fn description(&self) -> &str {
        &self.message
    }
}

pub fn tcp_check_host_and_port_used(bind_address: String) {
    if cfg!(target_os = "windows") {
        match std::net::TcpListener::bind(&bind_address) {
            Ok(e) => e,
            Err(_) => {
                panic!("Unable to bind to {} ! Exiting...", &bind_address);
            }
        };
    }
}

pub fn udp_check_host_and_port_used(bind_address: String) {
    if cfg!(target_os = "windows") {
        match std::net::UdpSocket::bind(&bind_address) {
            Ok(e) => e,
            Err(_) => {
                panic!("Unable to bind to {} ! Exiting...", &bind_address);
            }
        };
    }
}

#[derive(Deserialize, PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum AnnounceEvent {
    Started = 2,
    Stopped = 3,
    Completed = 1,
    None = 0,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "AnnounceEvent")]
pub enum AnnounceEventDef {
    Started,
    Stopped,
    Completed,
    None,
}

#[derive(PartialEq, PartialOrd, Eq, Hash, Clone, Copy, Debug)]
pub struct NumberOfBytes(pub i64);

#[derive(Serialize, Deserialize)]
#[serde(remote = "NumberOfBytes")]
pub struct NumberOfBytesDef(pub i64);

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
pub struct InfoHash(pub [u8; 20]);

impl fmt::Display for InfoHash {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        bin2hex(&self.0, f)
    }
}

impl std::str::FromStr for InfoHash {
    type Err = binascii::ConvertError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut i = Self([0u8; 20]);
        if s.len() != 40 {
            return Err(binascii::ConvertError::InvalidInputLength);
        }
        binascii::hex2bin(s.as_bytes(), &mut i.0)?;
        Ok(i)
    }
}

impl From<&[u8]> for InfoHash {
    fn from(data: &[u8]) -> InfoHash {
        assert_eq!(data.len(), 20);
        let mut ret = InfoHash([0u8; 20]);
        ret.0.clone_from_slice(data);
        ret
    }
}

impl From<[u8; 20]> for InfoHash {
    fn from(data: [u8; 20]) -> Self {
        InfoHash(data)
    }
}

impl serde::ser::Serialize for InfoHash {
    fn serialize<S: serde::ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut buffer = [0u8; 40];
        let bytes_out = binascii::bin2hex(&self.0, &mut buffer).ok().unwrap();
        let str_out = std::str::from_utf8(bytes_out).unwrap();
        serializer.serialize_str(str_out)
    }
}

impl<'de> serde::de::Deserialize<'de> for InfoHash {
    fn deserialize<D: serde::de::Deserializer<'de>>(des: D) -> Result<Self, D::Error> {
        des.deserialize_str(InfoHashVisitor)
    }
}

struct InfoHashVisitor;

impl<'v> serde::de::Visitor<'v> for InfoHashVisitor {
    type Value = InfoHash;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "a 40 character long hash")
    }

    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
        if v.len() != 40 {
            return Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(v),
                &"expected a 40 character long string",
            ));
        }

        let mut res = InfoHash([0u8; 20]);

        if binascii::hex2bin(v.as_bytes(), &mut res.0).is_err() {
            return Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(v),
                &"expected a hexadecimal string",
            ));
        } else {
            Ok(res)
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, PartialOrd, Ord)]
pub struct PeerId(pub [u8; 20]);

pub(crate) fn ser_instant<S: serde::Serializer>(inst: &std::time::Instant, ser: S) -> Result<S::Ok, S::Error> {
    ser.serialize_u64(inst.elapsed().as_millis() as u64)
}

impl fmt::Display for PeerId {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        bin2hex(&self.0, f)
    }
}

impl PeerId {
    pub fn get_client_name(&self) -> Option<&'static str> {
        if self.0[0] == b'M' {
            return Some("BitTorrent");
        }
        if self.0[0] == b'-' {
            let name = match &self.0[1..3] {
                b"AG" => "Ares",
                b"A~" => "Ares",
                b"AR" => "Arctic",
                b"AV" => "Avicora",
                b"AX" => "BitPump",
                b"AZ" => "Azureus",
                b"BB" => "BitBuddy",
                b"BC" => "BitComet",
                b"BF" => "Bitflu",
                b"BG" => "BTG (uses Rasterbar libtorrent)",
                b"BR" => "BitRocket",
                b"BS" => "BTSlave",
                b"BX" => "~Bittorrent X",
                b"CD" => "Enhanced CTorrent",
                b"CT" => "CTorrent",
                b"DE" => "DelugeTorrent",
                b"DP" => "Propagate Data Client",
                b"EB" => "EBit",
                b"ES" => "electric sheep",
                b"FT" => "FoxTorrent",
                b"FW" => "FrostWire",
                b"FX" => "Freebox BitTorrent",
                b"GS" => "GSTorrent",
                b"HL" => "Halite",
                b"HN" => "Hydranode",
                b"KG" => "KGet",
                b"KT" => "KTorrent",
                b"LH" => "LH-ABC",
                b"LP" => "Lphant",
                b"LT" => "libtorrent",
                b"lt" => "libTorrent",
                b"LW" => "LimeWire",
                b"MO" => "MonoTorrent",
                b"MP" => "MooPolice",
                b"MR" => "Miro",
                b"MT" => "MoonlightTorrent",
                b"NX" => "Net Transport",
                b"PD" => "Pando",
                b"PI" => "PicoTorrent",
                b"qB" => "qBittorrent",
                b"QD" => "QQDownload",
                b"QT" => "Qt 4 Torrent example",
                b"RT" => "Retriever",
                b"S~" => "Shareaza alpha/beta",
                b"SB" => "~Swiftbit",
                b"SS" => "SwarmScope",
                b"ST" => "SymTorrent",
                b"st" => "sharktorrent",
                b"SZ" => "Shareaza",
                b"TN" => "TorrentDotNET",
                b"TR" => "Transmission",
                b"TS" => "Torrentstorm",
                b"TT" => "TuoTu",
                b"UL" => "uLeecher!",
                b"UT" => "µTorrent",
                b"UW" => "µTorrent Web",
                b"VG" => "Vagaa",
                b"WD" => "WebTorrent Desktop",
                b"WT" => "BitLet",
                b"WW" => "WebTorrent",
                b"WY" => "FireTorrent",
                b"XL" => "Xunlei",
                b"XT" => "XanTorrent",
                b"XX" => "Xtorrent",
                b"ZT" => "ZipTorrent",
                _ => return None,
            };
            Some(name)
        } else {
            None
        }
    }
}

impl Serialize for PeerId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer, {
        let buff_size = self.0.len() * 2;
        let mut tmp: Vec<u8> = vec![0; buff_size];
        binascii::bin2hex(&self.0, &mut tmp).unwrap();
        let id = std::str::from_utf8(&tmp).ok();

        #[derive(Serialize)]
        struct PeerIdInfo<'a> {
            id: Option<&'a str>,
            client: Option<&'a str>,
        }

        let obj = PeerIdInfo {
            id,
            client: self.get_client_name(),
        };
        obj.serialize(serializer)
    }
}

impl std::str::FromStr for PeerId {
    type Err = binascii::ConvertError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut i = Self([0u8; 20]);
        if s.len() != 40 {
            return Err(binascii::ConvertError::InvalidInputLength);
        }
        binascii::hex2bin(s.as_bytes(), &mut i.0)?;
        Ok(i)
    }
}

impl From<&[u8]> for PeerId {
    fn from(data: &[u8]) -> PeerId {
        assert_eq!(data.len(), 20);
        let mut ret = PeerId([0u8; 20]);
        ret.0.clone_from_slice(data);
        ret
    }
}

impl<'de> serde::de::Deserialize<'de> for PeerId {
    fn deserialize<D: serde::de::Deserializer<'de>>(des: D) -> Result<Self, D::Error> {
        des.deserialize_str(PeerIdVisitor)
    }
}

struct PeerIdVisitor;

impl<'v> serde::de::Visitor<'v> for PeerIdVisitor {
    type Value = PeerId;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "a 40 character long hash")
    }

    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
        if v.len() != 40 {
            return Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(v),
                &"expected a 40 character long string",
            ));
        }

        let mut res = PeerId([0u8; 20]);

        if binascii::hex2bin(v.as_bytes(), &mut res.0).is_err() {
            return Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(v),
                &"expected a hexadecimal string",
            ));
        } else {
            Ok(res)
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Copy)]
pub struct TorrentPeer {
    pub peer_id: PeerId,
    pub peer_addr: SocketAddr,
    #[serde(serialize_with = "ser_instant")]
    pub updated: std::time::Instant,
    #[serde(with = "NumberOfBytesDef")]
    pub uploaded: NumberOfBytes,
    #[serde(with = "NumberOfBytesDef")]
    pub downloaded: NumberOfBytes,
    #[serde(with = "NumberOfBytesDef")]
    pub left: NumberOfBytes,
    #[serde(with = "AnnounceEventDef")]
    pub event: AnnounceEvent,
}

impl TorrentPeer {
    pub fn from_udp_announce_request(announce_request: &AnnounceRequest, remote_ip: IpAddr) -> Self {
        let peer_addr = TorrentPeer::peer_addr_from_ip_and_port_and_opt_host_ip(remote_ip, announce_request.port.0);

        let event = match announce_request.event {
            udp_common::AnnounceEvent::Started => { AnnounceEvent::Started }
            udp_common::AnnounceEvent::Stopped => { AnnounceEvent::Stopped }
            udp_common::AnnounceEvent::Completed => { AnnounceEvent::Completed }
            udp_common::AnnounceEvent::None => { AnnounceEvent::None }
        };
        TorrentPeer {
            peer_id: PeerId(announce_request.peer_id.0),
            peer_addr,
            updated: std::time::Instant::now(),
            uploaded: NumberOfBytes(announce_request.bytes_uploaded.0),
            downloaded: NumberOfBytes(announce_request.bytes_downloaded.0),
            left: NumberOfBytes(announce_request.bytes_left.0),
            event,
        }
    }

    // potentially substitute localhost ip with external ip
    pub fn peer_addr_from_ip_and_port_and_opt_host_ip(remote_ip: IpAddr, port: u16) -> SocketAddr {
        SocketAddr::new(remote_ip, port)
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
pub struct UserId(pub [u8; 20]);

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        bin2hex(&self.0, f)
    }
}

impl std::str::FromStr for UserId {
    type Err = binascii::ConvertError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut i = Self([0u8; 20]);
        if s.len() != 40 {
            return Err(binascii::ConvertError::InvalidInputLength);
        }
        binascii::hex2bin(s.as_bytes(), &mut i.0)?;
        Ok(i)
    }
}

impl From<&[u8]> for UserId {
    fn from(data: &[u8]) -> UserId {
        assert_eq!(data.len(), 20);
        let mut ret = UserId([0u8; 20]);
        ret.0.clone_from_slice(data);
        ret
    }
}

impl From<[u8; 20]> for UserId {
    fn from(data: [u8; 20]) -> Self {
        UserId(data)
    }
}

impl serde::ser::Serialize for UserId {
    fn serialize<S: serde::ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut buffer = [0u8; 40];
        let bytes_out = binascii::bin2hex(&self.0, &mut buffer).ok().unwrap();
        let str_out = std::str::from_utf8(bytes_out).unwrap();
        serializer.serialize_str(str_out)
    }
}

impl<'de> serde::de::Deserialize<'de> for UserId {
    fn deserialize<D: serde::de::Deserializer<'de>>(des: D) -> Result<Self, D::Error> {
        des.deserialize_str(UserIdVisitor)
    }
}

struct UserIdVisitor;

impl<'v> serde::de::Visitor<'v> for UserIdVisitor {
    type Value = UserId;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "a 40 character long hash")
    }

    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
        if v.len() != 40 {
            return Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(v),
                &"expected a 40 character long string",
            ));
        }

        let mut res = UserId([0u8; 20]);

        if binascii::hex2bin(v.as_bytes(), &mut res.0).is_err() {
            return Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(v),
                &"expected a hexadecimal string",
            ));
        } else {
            Ok(res)
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[allow(dead_code)]
pub struct AnnounceQueryRequest {
    pub(crate) info_hash: InfoHash,
    pub(crate) peer_id: PeerId,
    pub(crate) port: u16,
    pub(crate) uploaded: u64,
    pub(crate) downloaded: u64,
    pub(crate) left: u64,
    pub(crate) compact: bool,
    pub(crate) no_peer_id: bool,
    pub(crate) event: AnnounceEvent,
    pub(crate) remote_addr: IpAddr,
    pub(crate) numwant: u64,
}

#[derive(Deserialize, Clone, Debug)]
#[allow(dead_code)]
pub struct ScrapeQueryRequest {
    pub(crate) info_hash: Vec<InfoHash>,
}

pub(crate) fn bin2hex(data: &[u8; 20], f: &mut Formatter) -> fmt::Result {
    let mut chars = [0u8; 40];
    binascii::bin2hex(data, &mut chars).expect("failed to hexlify");
    write!(f, "{}", std::str::from_utf8(&chars).unwrap())
}

pub async fn maintenance_mode(tracker: Arc<TorrentTracker>) -> bool
{
    let stats = tracker.clone().get_stats().await;
    if stats.maintenance_mode != 0 {
        return true;
    }
    false
}