use std::sync::Arc;
use std::fmt;
use parking_lot::Mutex;

#[derive(Clone)]
pub struct CacheConnectorMemcache {
    pub(crate) client: Arc<Mutex<memcache::Client>>,
    pub(crate) prefix: String,
}

impl fmt::Debug for CacheConnectorMemcache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CacheConnectorMemcache")
            .field("client", &"<memcache::Client>")
            .field("prefix", &self.prefix)
            .finish()
    }
}