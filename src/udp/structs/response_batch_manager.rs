use crate::udp::structs::queued_response::QueuedResponse;
use mpsc::UnboundedSender;
use tokio::sync::mpsc;

pub struct ResponseBatchManager {
    pub(crate) sender: UnboundedSender<QueuedResponse>,
}