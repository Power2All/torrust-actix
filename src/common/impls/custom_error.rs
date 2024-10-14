use std::error::Error;
use std::fmt;
use std::fmt::Formatter;
use crate::common::structs::custom_error::CustomError;

impl CustomError {
    pub fn new(msg: &str) -> CustomError {
        CustomError { message: msg.to_string() }
    }
}

impl fmt::Display for CustomError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for CustomError {
    fn description(&self) -> &str {
        &self.message
    }
}