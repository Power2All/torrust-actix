use crate::seeder::structs::peer_conn::PeerConn;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type PeerMap = Arc<Mutex<HashMap<String, Arc<PeerConn>>>>;
pub const MSG_PIECE_REQUEST: u8 = 0x01;
pub const MSG_PIECE_DATA: u8 = 0x02;
pub const MSG_PIECE_CHUNK: u8 = 0x04;
pub const SCTP_MAX_PAYLOAD: usize = 65531;
pub const CHUNK_SIZE: usize = 16 * 1024;