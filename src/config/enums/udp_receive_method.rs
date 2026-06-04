use clap::ValueEnum;
use serde::{
    Deserialize,
    Serialize
};

#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum UdpReceiveMethod {
    #[default]
    recvmmsg,
    auto,
    io_uring,
    rio,
}