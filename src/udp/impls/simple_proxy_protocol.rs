use crate::udp::structs::simple_proxy_protocol::SppHeader;
use std::net::SocketAddr;

impl SppHeader {
    pub fn client_socket_addr(&self) -> SocketAddr {
        SocketAddr::new(self.client_addr, self.client_port)
    }

    pub fn proxy_socket_addr(&self) -> SocketAddr {
        SocketAddr::new(self.proxy_addr, self.proxy_port)
    }
}