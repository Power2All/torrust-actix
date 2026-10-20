use crate::common::common::bin2hex;
use crate::tracker::structs::user_id::UserId;
use std::fmt;
use std::fmt::Formatter;

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        bin2hex(&self.0, f)
    }
}

impl std::str::FromStr for UserId {
    type Err = crate::common::common::HexParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(UserId(crate::common::common::hex_to_id(s)?))
    }
}

impl From<&[u8]> for UserId {
    fn from(data: &[u8]) -> UserId {
        assert_eq!(data.len(), 20);
        let mut ret = UserId([0u8; 20]);
        ret.0.copy_from_slice(data);
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
        serializer.serialize_str(crate::common::common::bin20_to_hex(&self.0).as_str())
    }
}

impl<'de> serde::de::Deserialize<'de> for UserId {
    /// Parses the 40-character hex form [`Serialize`](serde::Serialize) writes, through `FromStr`.
    fn deserialize<D: serde::de::Deserializer<'de>>(des: D) -> Result<Self, D::Error> {
        <std::borrow::Cow<'de, str>>::deserialize(des)?.parse().map_err(serde::de::Error::custom)
    }
}
