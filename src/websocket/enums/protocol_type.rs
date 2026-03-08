use serde::{
    Deserialize,
    Serialize
};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ProtocolType {
    Http,
    Https,
    Udp,
    Api,
}