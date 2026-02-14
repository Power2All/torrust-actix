use crate::common::structs::number_of_bytes::NumberOfBytes;
use crate::common::structs::number_of_bytes_def::NumberOfBytesDef;
use crate::tracker::enums::announce_event::AnnounceEvent;
use crate::tracker::enums::announce_event_def::AnnounceEventDef;
use crate::tracker::structs::peer_id::PeerId;
use serde::{
    Deserialize,
    Serialize
};
use std::net::SocketAddr;

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct TorrentPeer {
    pub peer_id: PeerId,
    pub peer_addr: SocketAddr,
    #[serde(with = "serde_millis")]
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