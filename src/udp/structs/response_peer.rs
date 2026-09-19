use crate::udp::structs::port::Port;
use std::fmt::Debug;

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ResponsePeer<I: Clone + Copy + Debug + PartialEq + Eq> {
    pub ip_address: I,
    pub port: Port,
}