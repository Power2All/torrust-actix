use crate::udp::structs::simple_proxy_protocol::SppHeader;

#[derive(Debug)]
pub enum SppParseResult {
    Found {
        header: SppHeader,
        payload_offset: usize,
    },
    NotPresent,
    Malformed(String),
}
