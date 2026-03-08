use crate::websocket::structs::cluster_response::ClusterResponse;
use tokio::sync::oneshot;

pub type PendingRequestSender = oneshot::Sender<ClusterResponse>;
pub type SlaveSenderChannel = parking_lot::RwLock<Option<tokio::sync::mpsc::UnboundedSender<Vec<u8>>>>;