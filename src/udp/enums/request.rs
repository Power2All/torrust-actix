use crate::udp::structs::announce_request::AnnounceRequest;
use crate::udp::structs::connect_request::ConnectRequest;
use crate::udp::structs::scrape_request::ScrapeRequest;

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Request {
    Connect(ConnectRequest),
    Announce(AnnounceRequest),
    Scrape(ScrapeRequest),
}