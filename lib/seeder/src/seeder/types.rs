use crate::seeder::structs::peer_conn::PeerConn;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub const BT_HANDSHAKE_LEN: usize = 68;
pub const BT_PROTOCOL: &[u8] = b"\x13BitTorrent protocol";
pub const MSG_CHOKE: u8 = 0;
pub const MSG_UNCHOKE: u8 = 1;
pub const MSG_INTERESTED: u8 = 2;
pub const MSG_NOT_INTERESTED: u8 = 3;
pub const MSG_HAVE: u8 = 4;
pub const MSG_BITFIELD: u8 = 5;
pub const MSG_REQUEST: u8 = 6;
pub const MSG_PIECE: u8 = 7;
pub const MSG_CANCEL: u8 = 8;
pub const MAX_BLOCK_SIZE: u32 = 16 * 1024;
pub const MSG_PIECE_REQUEST: u8 = 0x01;
pub const MSG_PIECE_DATA: u8 = 0x02;
pub const MSG_PIECE_CHUNK: u8 = 0x04;
pub const SCTP_MAX_PAYLOAD: usize = 65531;
pub const CHUNK_SIZE: usize = 16 * 1024;

pub type PeerMap = Arc<Mutex<HashMap<String, Arc<PeerConn>>>>;