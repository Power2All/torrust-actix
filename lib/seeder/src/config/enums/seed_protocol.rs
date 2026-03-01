use serde::{
    Deserialize,
    Serialize
};

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SeedProtocol {
    Bt,
    Rtc,
    #[default]
    Both,
}