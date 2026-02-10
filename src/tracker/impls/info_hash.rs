use std::fmt;
use std::fmt::Formatter;
use crate::common::common::bin2hex;
use crate::common::common::hex_to_nibble;
use crate::tracker::structs::info_hash::InfoHash;

impl fmt::Display for InfoHash {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        bin2hex(&self.0, f)
    }
}

impl std::str::FromStr for InfoHash {
    type Err = binascii::ConvertError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 40 {
            return Err(binascii::ConvertError::InvalidInputLength);
        }
        let mut result = InfoHash([0u8; 20]);
        let bytes = s.as_bytes();
        for (i, chunk) in bytes.chunks_exact(2).enumerate() {
            let high = hex_to_nibble(chunk[0]);
            let low = hex_to_nibble(chunk[1]);
            if high == 0xFF || low == 0xFF {
                return Err(binascii::ConvertError::InvalidInput);
            }
            result.0[i] = (high << 4) | low;
        }
        Ok(result)
    }
}

impl From<&[u8]> for InfoHash {
    fn from(data: &[u8]) -> InfoHash {
        assert_eq!(data.len(), 20);
        let mut ret = InfoHash([0u8; 20]);
        ret.0.copy_from_slice(data);
        ret
    }
}

impl From<[u8; 20]> for InfoHash {
    fn from(data: [u8; 20]) -> Self {
        InfoHash(data)
    }
}

impl serde::ser::Serialize for InfoHash {
    fn serialize<S: serde::ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
        let mut buffer = [0u8; 40];
        for (i, &byte) in self.0.iter().enumerate() {
            let idx = i * 2;
            buffer[idx] = HEX_CHARS[(byte >> 4) as usize];
            buffer[idx + 1] = HEX_CHARS[(byte & 0xf) as usize];
        }
        let str_out = unsafe { std::str::from_utf8_unchecked(&buffer) };
        serializer.serialize_str(str_out)
    }
}

impl<'de> serde::de::Deserialize<'de> for InfoHash {
    fn deserialize<D: serde::de::Deserializer<'de>>(des: D) -> Result<Self, D::Error> {
        struct InfoHashVisitor;

        impl<'de> serde::de::Visitor<'de> for InfoHashVisitor {
            type Value = InfoHash;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a 40 character hex string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if v.len() != 40 {
                    return Err(E::custom("expected 40 character hex string"));
                }
                let mut result = InfoHash([0u8; 20]);
                let bytes = v.as_bytes();
                for (i, chunk) in bytes.chunks_exact(2).enumerate() {
                    let high = hex_to_nibble(chunk[0]);
                    let low = hex_to_nibble(chunk[1]);
                    if high == 0xFF || low == 0xFF {
                        return Err(E::custom("invalid hex character"));
                    }
                    result.0[i] = (high << 4) | low;
                }
                Ok(result)
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if v.len() != 40 {
                    return Err(E::custom("expected 40 byte hex string"));
                }
                let mut result = InfoHash([0u8; 20]);
                for (i, chunk) in v.chunks_exact(2).enumerate() {
                    let high = hex_to_nibble(chunk[0]);
                    let low = hex_to_nibble(chunk[1]);
                    if high == 0xFF || low == 0xFF {
                        return Err(E::custom("invalid hex character"));
                    }
                    result.0[i] = (high << 4) | low;
                }
                Ok(result)
            }
        }
        des.deserialize_str(InfoHashVisitor)
    }
}