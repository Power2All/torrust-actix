use crate::webtorrent::structs::wt_scrape_info::WtScrapeInfo;
use serde::{
    Deserialize,
    Serialize
};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WtScrapeResponse {
    pub files: HashMap<String, WtScrapeInfo>,
}