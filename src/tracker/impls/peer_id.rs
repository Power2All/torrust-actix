use std::fmt;
use std::fmt::Formatter;
use serde::Serialize;
use crate::common::common::bin2hex;
use crate::tracker::structs::peer_id::PeerId;

impl fmt::Display for PeerId {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        bin2hex(&self.0, f)
    }
}

impl PeerId {
    pub fn get_client_name(&self) -> Option<&'static str> {
        if self.0[0] == b'M' {
            return Some("BitTorrent");
        }
        if self.0[0] == b'-' {
            let name = match &self.0[1..3] {
                b"AG" => "Ares",
                b"A~" => "Ares",
                b"AR" => "Arctic",
                b"AV" => "Avicora",
                b"AX" => "BitPump",
                b"AZ" => "Azureus",
                b"BB" => "BitBuddy",
                b"BC" => "BitComet",
                b"BF" => "Bitflu",
                b"BG" => "BTG (uses Rasterbar libtorrent)",
                b"BR" => "BitRocket",
                b"BS" => "BTSlave",
                b"BX" => "~Bittorrent X",
                b"CD" => "Enhanced CTorrent",
                b"CT" => "CTorrent",
                b"DE" => "DelugeTorrent",
                b"DP" => "Propagate Data Client",
                b"EB" => "EBit",
                b"ES" => "electric sheep",
                b"FT" => "FoxTorrent",
                b"FW" => "FrostWire",
                b"FX" => "Freebox BitTorrent",
                b"GS" => "GSTorrent",
                b"HL" => "Halite",
                b"HN" => "Hydranode",
                b"KG" => "KGet",
                b"KT" => "KTorrent",
                b"LH" => "LH-ABC",
                b"LP" => "Lphant",
                b"LT" => "libtorrent",
                b"lt" => "libTorrent",
                b"LW" => "LimeWire",
                b"MO" => "MonoTorrent",
                b"MP" => "MooPolice",
                b"MR" => "Miro",
                b"MT" => "MoonlightTorrent",
                b"NX" => "Net Transport",
                b"PD" => "Pando",
                b"PI" => "PicoTorrent",
                b"qB" => "qBittorrent",
                b"QD" => "QQDownload",
                b"QT" => "Qt 4 Torrent example",
                b"RT" => "Retriever",
                b"S~" => "Shareaza alpha/beta",
                b"SB" => "~Swiftbit",
                b"SS" => "SwarmScope",
                b"ST" => "SymTorrent",
                b"st" => "sharktorrent",
                b"SZ" => "Shareaza",
                b"TN" => "TorrentDotNET",
                b"TR" => "Transmission",
                b"TS" => "Torrentstorm",
                b"TT" => "TuoTu",
                b"UL" => "uLeecher!",
                b"UT" => "µTorrent",
                b"UW" => "µTorrent Web",
                b"VG" => "Vagaa",
                b"WD" => "WebTorrent Desktop",
                b"WT" => "BitLet",
                b"WW" => "WebTorrent",
                b"WY" => "FireTorrent",
                b"XL" => "Xunlei",
                b"XT" => "XanTorrent",
                b"XX" => "Xtorrent",
                b"ZT" => "ZipTorrent",
                _ => return None,
            };
            Some(name)
        } else {
            None
        }
    }
}

impl Serialize for PeerId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
        let mut buffer = [0u8; 40];

        for (i, &byte) in self.0.iter().enumerate() {
            buffer[i * 2] = HEX_CHARS[(byte >> 4) as usize];
            buffer[i * 2 + 1] = HEX_CHARS[(byte & 0xf) as usize];
        }

        // SAFETY: We know the buffer contains only valid ASCII hex characters
        let id = unsafe { std::str::from_utf8_unchecked(&buffer) };

        #[derive(Serialize)]
        struct PeerIdInfo<'a> {
            id: &'a str,
            client: Option<&'a str>,
        }

        let obj = PeerIdInfo {
            id,
            client: self.get_client_name(),
        };
        obj.serialize(serializer)
    }
}

impl std::str::FromStr for PeerId {
    type Err = binascii::ConvertError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 40 {
            return Err(binascii::ConvertError::InvalidInputLength);
        }

        let mut result = PeerId([0u8; 20]);
        let bytes = s.as_bytes();

        for i in 0..20 {
            let high = hex_char_to_nibble(bytes[i * 2]);
            let low = hex_char_to_nibble(bytes[i * 2 + 1]);

            if high == 0xFF || low == 0xFF {
                return Err(binascii::ConvertError::InvalidInput);
            }

            result.0[i] = (high << 4) | low;
        }

        Ok(result)
    }
}

impl From<&[u8]> for PeerId {
    fn from(data: &[u8]) -> PeerId {
        assert_eq!(data.len(), 20);
        let mut ret = PeerId([0u8; 20]);
        ret.0.copy_from_slice(data);
        ret
    }
}

impl<'de> serde::de::Deserialize<'de> for PeerId {
    fn deserialize<D: serde::de::Deserializer<'de>>(des: D) -> Result<Self, D::Error> {
        struct PeerIdVisitor;

        impl<'de> serde::de::Visitor<'de> for PeerIdVisitor {
            type Value = PeerId;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a 40 character long hash")
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

                let mut res = PeerId([0u8; 20]);
                let bytes = v.as_bytes();

                for i in 0..20 {
                    let high = hex_char_to_nibble(bytes[i * 2]);
                    let low = hex_char_to_nibble(bytes[i * 2 + 1]);

                    if high == 0xFF || low == 0xFF {
                        return Err(serde::de::Error::invalid_value(
                            serde::de::Unexpected::Str(v),
                            &"expected a hexadecimal string",
                        ));
                    }

                    res.0[i] = (high << 4) | low;
                }

                Ok(res)
            }
        }

        des.deserialize_str(PeerIdVisitor)
    }
}

#[inline(always)]
fn hex_char_to_nibble(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => 0xFF,
    }
}