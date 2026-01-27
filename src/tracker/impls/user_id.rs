use std::fmt;
use std::fmt::Formatter;
use crate::common::common::bin2hex;
use crate::tracker::structs::user_id::UserId;

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        bin2hex(&self.0, f)
    }
}

impl std::str::FromStr for UserId {
    type Err = binascii::ConvertError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 40 {
            return Err(binascii::ConvertError::InvalidInputLength);
        }

        let mut result = UserId([0u8; 20]);
        let bytes = s.as_bytes();

        for i in 0..20 {
            let high = hex_to_nibble(bytes[i * 2]);
            let low = hex_to_nibble(bytes[i * 2 + 1]);

            if high == 0xFF || low == 0xFF {
                return Err(binascii::ConvertError::InvalidInput);
            }

            result.0[i] = (high << 4) | low;
        }

        Ok(result)
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
        const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
        let mut buffer = [0u8; 40];

        for (i, &byte) in self.0.iter().enumerate() {
            buffer[i * 2] = HEX_CHARS[(byte >> 4) as usize];
            buffer[i * 2 + 1] = HEX_CHARS[(byte & 0xf) as usize];
        }

        
        let str_out = unsafe { std::str::from_utf8_unchecked(&buffer) };
        serializer.serialize_str(str_out)
    }
}

impl<'de> serde::de::Deserialize<'de> for UserId {
    fn deserialize<D: serde::de::Deserializer<'de>>(des: D) -> Result<Self, D::Error> {
        struct UserIdVisitor;

        impl<'de> serde::de::Visitor<'de> for UserIdVisitor {
            type Value = UserId;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a 40 character hex string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if v.len() != 40 {
                    return Err(serde::de::Error::invalid_value(
                        serde::de::Unexpected::Str(v),
                        &"expected a 40 character long string",
                    ));
                }

                let mut result = UserId([0u8; 20]);
                let bytes = v.as_bytes();

                for i in 0..20 {
                    let high = hex_to_nibble(bytes[i * 2]);
                    let low = hex_to_nibble(bytes[i * 2 + 1]);

                    if high == 0xFF || low == 0xFF {
                        return Err(serde::de::Error::invalid_value(
                            serde::de::Unexpected::Str(v),
                            &"expected a hexadecimal string",
                        ));
                    }

                    result.0[i] = (high << 4) | low;
                }

                Ok(result)
            }
        }

        des.deserialize_str(UserIdVisitor)
    }
}

#[inline(always)]
fn hex_to_nibble(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => 0xFF,
    }
}