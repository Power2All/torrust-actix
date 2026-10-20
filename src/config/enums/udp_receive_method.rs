use clap::ValueEnum;
use serde::{
    Deserialize,
    Serialize
};

#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum UdpReceiveMethod {
    /// `auto` is still accepted for configs written against earlier releases; it always meant this.
    #[default]
    #[serde(alias = "auto")]
    #[value(alias = "auto")]
    recvmmsg,
    io_uring,
    rio,
}