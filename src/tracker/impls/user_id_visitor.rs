use std::fmt;
use std::fmt::Formatter;
use crate::tracker::structs::user_id::UserId;
use crate::tracker::structs::user_id_visitor::UserIdVisitor;

impl serde::de::Visitor<'_> for UserIdVisitor {
    type Value = UserId;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "a 40 character long hash")
    }

    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
        if v.len() != 40 {
            return Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(v),
                &"expected a 40 character long string",
            ));
        }

        let mut res = UserId([0u8; 20]);

        if binascii::hex2bin(v.as_bytes(), &mut res.0).is_err() {
            Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(v),
                &"expected a hexadecimal string",
            ))
        } else {
            Ok(res)
        }
    }
}