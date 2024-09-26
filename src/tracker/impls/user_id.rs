use std::fmt;
use std::fmt::Formatter;
use crate::common::common::bin2hex;
use crate::tracker::structs::user_id::UserId;
use crate::tracker::structs::user_id_visitor::UserIdVisitor;

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        bin2hex(&self.0, f)
    }
}

impl std::str::FromStr for UserId {
    type Err = binascii::ConvertError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut i = Self([0u8; 20]);
        if s.len() != 40 {
            return Err(binascii::ConvertError::InvalidInputLength);
        }
        binascii::hex2bin(s.as_bytes(), &mut i.0)?;
        Ok(i)
    }
}

impl From<&[u8]> for UserId {
    fn from(data: &[u8]) -> UserId {
        assert_eq!(data.len(), 20);
        let mut ret = UserId([0u8; 20]);
        ret.0.clone_from_slice(data);
        ret
    }
}

impl From<[u8; 20]> for UserId {
    fn from(data: [u8; 20]) -> Self {
        UserId(data)
    }
}

impl serde::ser::Serialize for UserId {
    fn serialize<S: serde::ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut buffer = [0u8; 40];
        let bytes_out = binascii::bin2hex(&self.0, &mut buffer).ok().unwrap();
        let str_out = std::str::from_utf8(bytes_out).unwrap();
        serializer.serialize_str(str_out)
    }
}

impl<'de> serde::de::Deserialize<'de> for UserId {
    fn deserialize<D: serde::de::Deserializer<'de>>(des: D) -> Result<Self, D::Error> {
        des.deserialize_str(UserIdVisitor)
    }
}