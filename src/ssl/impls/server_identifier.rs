use crate::ssl::enums::server_identifier::ServerIdentifier;

impl ServerIdentifier {
    pub fn bind_address(&self) -> &str {
        match self {
            ServerIdentifier::HttpTracker(addr) => addr,
            ServerIdentifier::ApiServer(addr) => addr,
            ServerIdentifier::WebSocketMaster(addr) => addr,
        }
    }

    pub fn server_type(&self) -> &'static str {
        match self {
            ServerIdentifier::HttpTracker(_) => "http",
            ServerIdentifier::ApiServer(_) => "api",
            ServerIdentifier::WebSocketMaster(_) => "websocket",
        }
    }
}

impl std::fmt::Display for ServerIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerIdentifier::HttpTracker(addr) => {
                write!(f, "HttpTracker({})", addr)
            }
            ServerIdentifier::ApiServer(addr) => {
                write!(f, "ApiServer({})", addr)
            }
            ServerIdentifier::WebSocketMaster(addr) => {
                write!(f, "WebSocketMaster({})", addr)
            }
        }
    }
}