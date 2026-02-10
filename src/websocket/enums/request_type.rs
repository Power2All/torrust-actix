use serde::{
    Deserialize,
    Serialize
};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum RequestType {
    Announce,
    Scrape,
    ApiCall {
        endpoint: String,
        method: String,
    },
    UdpPacket,
    WtAnnounce,
    WtScrape,
}