use crate::udp::structs::port::Port;
use crate::udp::traits::Ip;

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ResponsePeer<I: Ip> {
    pub ip_address: I,
    pub port: Port,
}
