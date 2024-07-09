use std::fmt;
use std::fmt::Formatter;
use crate::tracker::structs::peer_id::PeerId;
use crate::tracker::structs::peer_id_visitor::PeerIdVisitor;

impl<'v> serde::de::Visitor<'v> for PeerIdVisitor {
    type Value = PeerId;

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

        let mut res = PeerId([0u8; 20]);

        if binascii::hex2bin(v.as_bytes(), &mut res.0).is_err() {
            return Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(v),
                &"expected a hexadecimal string",
            ));
        } else {
            Ok(res)
        }
    }
}
