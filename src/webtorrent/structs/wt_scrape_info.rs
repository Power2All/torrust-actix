use serde::{
    Deserialize,
    Serialize
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WtScrapeInfo {
    pub complete: i64,
    pub downloaded: i64,
    pub incomplete: i64,
}