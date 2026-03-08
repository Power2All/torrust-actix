use std::net::IpAddr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SppHeader {
    pub client_addr: IpAddr,
    pub client_port: u16,
    pub proxy_addr: IpAddr,
    pub proxy_port: u16,
}