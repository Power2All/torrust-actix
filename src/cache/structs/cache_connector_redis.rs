use redis::aio::MultiplexedConnection;

#[derive(Debug, Clone)]
pub struct CacheConnectorRedis {
    pub(crate) connection: MultiplexedConnection,
    pub(crate) prefix: String,
}