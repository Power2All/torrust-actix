use clap::ValueEnum;
use serde::{
    Deserialize,
    Serialize
};

#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Default)]
pub enum ClusterEncoding {
    #[default]
    binary,
    json,
    msgpack,
}