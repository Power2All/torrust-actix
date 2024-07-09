use std::net::{Ipv4Addr, Ipv6Addr};
use crate::udp::structs::announce_response::AnnounceResponse;
use crate::udp::structs::connect_response::ConnectResponse;
use crate::udp::structs::error_response::ErrorResponse;
use crate::udp::structs::scrape_response::ScrapeResponse;

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Response {
    Connect(ConnectResponse),
    AnnounceIpv4(AnnounceResponse<Ipv4Addr>),
    AnnounceIpv6(AnnounceResponse<Ipv6Addr>),
    Scrape(ScrapeResponse),
    Error(ErrorResponse),
}
