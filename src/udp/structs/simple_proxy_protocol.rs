use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

pub const SPP_HEADER_SIZE: usize = 38;

pub const SPP_MAGIC: u16 = 0x56EC;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SppHeader {
    pub client_addr: IpAddr,
    pub client_port: u16,
    pub proxy_addr: IpAddr,
    pub proxy_port: u16,
}

impl SppHeader {
    pub fn client_socket_addr(&self) -> SocketAddr {
        SocketAddr::new(self.client_addr, self.client_port)
    }

    pub fn proxy_socket_addr(&self) -> SocketAddr {
        SocketAddr::new(self.proxy_addr, self.proxy_port)
    }
}

#[derive(Debug)]
pub enum SppParseResult {
    Found {
        header: SppHeader,
        payload_offset: usize,
    },
    NotPresent,
    Malformed(String),
}

fn parse_address(bytes: &[u8; 16]) -> IpAddr {
    let is_ipv4_mapped = bytes[0..10] == [0u8; 10] && bytes[10] == 0xff && bytes[11] == 0xff;
    if is_ipv4_mapped {
        IpAddr::V4(Ipv4Addr::new(bytes[12], bytes[13], bytes[14], bytes[15]))
    } else {
        IpAddr::V6(Ipv6Addr::from(*bytes))
    }
}

pub fn parse_spp_header(data: &[u8]) -> SppParseResult {
    if data.len() < 2 {
        return SppParseResult::NotPresent;
    }
    let magic = u16::from_be_bytes([data[0], data[1]]);
    if magic != SPP_MAGIC {
        return SppParseResult::NotPresent;
    }
    if data.len() < SPP_HEADER_SIZE {
        return SppParseResult::Malformed(format!(
            "SPP magic found but packet too small: {} bytes, need {}",
            data.len(),
            SPP_HEADER_SIZE
        ));
    }
    let client_addr_bytes: [u8; 16] = data[2..18]
        .try_into()
        .expect("slice with correct length");
    let client_addr = parse_address(&client_addr_bytes);
    let proxy_addr_bytes: [u8; 16] = data[18..34]
        .try_into()
        .expect("slice with correct length");
    let proxy_addr = parse_address(&proxy_addr_bytes);
    let client_port = u16::from_be_bytes([data[34], data[35]]);
    let proxy_port = u16::from_be_bytes([data[36], data[37]]);
    SppParseResult::Found {
        header: SppHeader {
            client_addr,
            client_port,
            proxy_addr,
            proxy_port,
        },
        payload_offset: SPP_HEADER_SIZE,
    }
}

#[inline]
pub fn has_spp_magic(data: &[u8]) -> bool {
    data.len() >= 2 && data[0] == 0x56 && data[1] == 0xEC
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_spp_header(
        client_ip: IpAddr,
        proxy_ip: IpAddr,
        client_port: u16,
        proxy_port: u16,
    ) -> Vec<u8> {
        let mut header = Vec::with_capacity(SPP_HEADER_SIZE);
        header.extend_from_slice(&SPP_MAGIC.to_be_bytes());
        match client_ip {
            IpAddr::V4(ipv4) => {
                // IPv4-mapped IPv6: ::ffff:x.x.x.x
                header.extend_from_slice(&[0u8; 10]);
                header.extend_from_slice(&[0xff, 0xff]);
                header.extend_from_slice(&ipv4.octets());
            }
            IpAddr::V6(ipv6) => {
                header.extend_from_slice(&ipv6.octets());
            }
        }
        match proxy_ip {
            IpAddr::V4(ipv4) => {
                header.extend_from_slice(&[0u8; 10]);
                header.extend_from_slice(&[0xff, 0xff]);
                header.extend_from_slice(&ipv4.octets());
            }
            IpAddr::V6(ipv6) => {
                header.extend_from_slice(&ipv6.octets());
            }
        }
        header.extend_from_slice(&client_port.to_be_bytes());
        header.extend_from_slice(&proxy_port.to_be_bytes());
        assert_eq!(header.len(), SPP_HEADER_SIZE);
        header
    }

    #[test]
    fn test_spp_header_size() {
        assert_eq!(SPP_HEADER_SIZE, 38);
    }

    #[test]
    fn test_spp_magic() {
        assert_eq!(SPP_MAGIC, 0x56EC);
    }

    #[test]
    fn test_has_spp_magic() {
        assert!(has_spp_magic(&[0x56, 0xEC]));
        assert!(has_spp_magic(&[0x56, 0xEC, 0x00, 0x00]));
        assert!(!has_spp_magic(&[0x00, 0x00]));
        assert!(!has_spp_magic(&[0x56]));
        assert!(!has_spp_magic(&[]));
    }

    #[test]
    fn test_parse_spp_header_ipv4() {
        let client_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));
        let proxy_ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        let client_port = 12345u16;
        let proxy_port = 443u16;
        let header = create_spp_header(client_ip, proxy_ip, client_port, proxy_port);
        match parse_spp_header(&header) {
            SppParseResult::Found {
                header: spp,
                payload_offset,
            } => {
                assert_eq!(spp.client_addr, client_ip);
                assert_eq!(spp.proxy_addr, proxy_ip);
                assert_eq!(spp.client_port, client_port);
                assert_eq!(spp.proxy_port, proxy_port);
                assert_eq!(payload_offset, SPP_HEADER_SIZE);
            }
            other => panic!("Expected Found, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_spp_header_ipv6() {
        let client_ip = IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
        let proxy_ip = IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 2));
        let client_port = 54321u16;
        let proxy_port = 8080u16;
        let header = create_spp_header(client_ip, proxy_ip, client_port, proxy_port);
        match parse_spp_header(&header) {
            SppParseResult::Found {
                header: spp,
                payload_offset,
            } => {
                assert_eq!(spp.client_addr, client_ip);
                assert_eq!(spp.proxy_addr, proxy_ip);
                assert_eq!(spp.client_port, client_port);
                assert_eq!(spp.proxy_port, proxy_port);
                assert_eq!(payload_offset, SPP_HEADER_SIZE);
            }
            other => panic!("Expected Found, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_spp_header_with_payload() {
        let client_ip = IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8));
        let proxy_ip = IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1));
        let mut packet = create_spp_header(client_ip, proxy_ip, 6881, 443);
        packet.extend_from_slice(&[0x00, 0x00, 0x04, 0x17, 0x27, 0x10, 0x19, 0x80]); // connection_id
        match parse_spp_header(&packet) {
            SppParseResult::Found {
                header: spp,
                payload_offset,
            } => {
                assert_eq!(spp.client_addr, client_ip);
                assert_eq!(payload_offset, SPP_HEADER_SIZE);
                // Verify we can access the payload
                let payload = &packet[payload_offset..];
                assert_eq!(payload.len(), 8);
            }
            other => panic!("Expected Found, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_spp_header_not_present_wrong_magic() {
        let data = [
            0x00, 0x00, 0x04, 0x17, 0x27, 0x10, 0x19, 0x80,
            0x00, 0x00, 0x00, 0x00,
            0x12, 0x34, 0x56, 0x78,
        ];
        match parse_spp_header(&data) {
            SppParseResult::NotPresent => {}
            other => panic!("Expected NotPresent, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_spp_header_not_present_empty() {
        match parse_spp_header(&[]) {
            SppParseResult::NotPresent => {}
            other => panic!("Expected NotPresent, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_spp_header_not_present_too_small() {
        match parse_spp_header(&[0x56]) {
            SppParseResult::NotPresent => {}
            other => panic!("Expected NotPresent, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_spp_header_malformed_incomplete() {
        let data = [0x56, 0xEC, 0x00, 0x00, 0x00];
        match parse_spp_header(&data) {
            SppParseResult::Malformed(msg) => {
                assert!(msg.contains("too small"));
            }
            other => panic!("Expected Malformed, got {:?}", other),
        }
    }

    #[test]
    fn test_spp_header_client_socket_addr() {
        let header = SppHeader {
            client_addr: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            client_port: 12345,
            proxy_addr: IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            proxy_port: 443,
        };
        let socket_addr = header.client_socket_addr();
        assert_eq!(socket_addr.ip(), IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)));
        assert_eq!(socket_addr.port(), 12345);
    }

    #[test]
    fn test_spp_header_proxy_socket_addr() {
        let header = SppHeader {
            client_addr: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            client_port: 12345,
            proxy_addr: IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            proxy_port: 443,
        };
        let socket_addr = header.proxy_socket_addr();
        assert_eq!(socket_addr.ip(), IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)));
        assert_eq!(socket_addr.port(), 443);
    }

    #[test]
    fn test_parse_ipv4_mapped_address() {
        let bytes: [u8; 16] = [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 192, 168, 1,
            100,
        ];
        let addr = parse_address(&bytes);
        assert_eq!(addr, IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)));
    }

    #[test]
    fn test_parse_native_ipv6_address() {
        let bytes: [u8; 16] = [
            0x20, 0x01, 0x0d, 0xb8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x01,
        ];
        let addr = parse_address(&bytes);
        assert_eq!(
            addr,
            IpAddr::V6(Ipv6Addr::new(0x2001, 0x0db8, 0, 0, 0, 0, 0, 1))
        );
    }

    #[test]
    fn test_edge_case_ports() {
        let client_ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let proxy_ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let header = create_spp_header(client_ip, proxy_ip, 0, 0);
        if let SppParseResult::Found { header: spp, .. } = parse_spp_header(&header) {
            assert_eq!(spp.client_port, 0);
            assert_eq!(spp.proxy_port, 0);
        }
        let header = create_spp_header(client_ip, proxy_ip, 65535, 65535);
        if let SppParseResult::Found { header: spp, .. } = parse_spp_header(&header) {
            assert_eq!(spp.client_port, 65535);
            assert_eq!(spp.proxy_port, 65535);
        }
    }
}