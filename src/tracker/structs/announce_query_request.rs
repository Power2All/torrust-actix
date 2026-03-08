use crate::tracker::enums::announce_event::AnnounceEvent;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::peer_id::PeerId;
use serde::Deserialize;
use std::net::IpAddr;

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
    pub(crate) rtctorrent: Option<bool>,
    pub(crate) rtcoffer: Option<String>,
    pub(crate) rtcrequest: Option<bool>,
    pub(crate) rtcanswer: Option<String>,
    pub(crate) rtcanswerfor: Option<String>,
}