use parking_lot::Mutex;
use std::sync::Arc;

#[derive(Clone)]
pub struct CacheConnectorMemcache {
    pub(crate) client: Arc<Mutex<memcache::Client>>,
    pub(crate) prefix: String,
}