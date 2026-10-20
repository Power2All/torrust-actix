use crate::common::structs::custom_error::CustomError;

impl CustomError {
    /// Creates an error carrying the given message.
    pub fn new(msg: &str) -> CustomError {
        CustomError { message: msg.to_string() }
    }
}
