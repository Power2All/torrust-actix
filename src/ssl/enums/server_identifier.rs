#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum ServerIdentifier {
    HttpTracker(String),
    ApiServer(String),
    WebSocketMaster(String),
    WebTorrentTracker(String),
}