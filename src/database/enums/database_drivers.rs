use clap::ValueEnum;
use serde::{
    Deserialize,
    Serialize
};

#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum DatabaseDrivers {
    sqlite3,
    mysql,
    pgsql,
}