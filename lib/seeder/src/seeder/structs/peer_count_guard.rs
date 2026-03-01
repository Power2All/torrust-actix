use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

pub(crate) struct PeerCountGuard {
    pub(crate) count: Arc<AtomicUsize>,
}