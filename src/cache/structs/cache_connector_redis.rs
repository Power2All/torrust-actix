use redis::Client;

#[derive(Debug, Clone)]
pub struct CacheConnectorRedis {
    pub(crate) client: Client,
    pub(crate) prefix: String,
    pub(crate) split_peers: bool,
}
