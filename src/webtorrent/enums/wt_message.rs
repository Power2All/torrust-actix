use crate::webtorrent::structs::wt_announce::WtAnnounce;
use crate::webtorrent::structs::wt_answer::WtAnswer;
use crate::webtorrent::structs::wt_offer::WtOffer;
use crate::webtorrent::structs::wt_scrape::WtScrape;
use serde::{
    Deserialize,
    Serialize
};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "action")]
pub enum WtMessage {
    #[serde(rename = "announce")]
    Announce(WtAnnounce),
    #[serde(rename = "scrape")]
    Scrape(WtScrape),
    #[serde(rename = "offer")]
    Offer(WtOffer),
    #[serde(rename = "answer")]
    Answer(WtAnswer),
}