use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use tokio::sync::mpsc;
use crate::udp::structs::udp_packet::UdpPacket;

/// Dynamic ParsePool that can scale worker threads based on load
///
/// Uses an unbounded channel to prevent packet drops during traffic bursts.
/// The pool automatically spawns additional "burst" workers when the queue
/// depth exceeds thresholds, and these workers exit when idle.
pub struct ParsePool {
    /// Sender for pushing packets into the queue (cloneable for multiple UDP threads)
    pub sender: Arc<mpsc::UnboundedSender<UdpPacket>>,

    /// Receiver for pulling packets from the queue (protected by mutex for sharing)
    pub receiver: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<UdpPacket>>>,

    /// Approximate queue length counter (atomic for lock-free access)
    pub(crate) queue_len: Arc<AtomicUsize>,

    /// Max burst workers
    pub(crate) max_burst: usize,

    /// Queue threshold for spawning new workers
    pub(crate) queue_threshold: usize,

    /// Low threshold for spawning new workers
    pub(crate) low_threshold: usize,
}