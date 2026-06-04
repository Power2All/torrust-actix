use std::sync::Arc;
use tokio::net::UdpSocket;

#[derive(Debug, Clone)]
pub enum UdpReply {
    Socket(Arc<UdpSocket>),
    #[cfg(windows)]
    Rio(Arc<crate::udp::impls::rio_recv::RioSender>),
}