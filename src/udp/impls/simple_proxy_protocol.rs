use crate::udp::structs::simple_proxy_protocol::SppHeader;
use std::net::SocketAddr;

impl SppHeader {
    /// Returns the real client address carried in the Simple Proxy Protocol header.
    pub fn client_socket_addr(&self) -> SocketAddr {
        SocketAddr::new(self.client_addr, self.client_port)
    }

    /// Returns the proxy address that forwarded the datagram.
    pub fn proxy_socket_addr(&self) -> SocketAddr {
        SocketAddr::new(self.proxy_addr, self.proxy_port)
    }
}