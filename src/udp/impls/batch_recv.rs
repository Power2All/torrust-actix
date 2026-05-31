use crate::udp::udp::MAX_PACKET_SIZE;
use std::net::{
    IpAddr,
    Ipv4Addr,
    Ipv6Addr,
    SocketAddr
};
use std::os::unix::io::RawFd;

pub const BATCH: usize = 64;

pub struct RecvBatch {
    bufs: Box<[[u8; MAX_PACKET_SIZE]; BATCH]>,
    addrs: Box<[libc::sockaddr_storage; BATCH]>,
    iovecs: Box<[libc::iovec; BATCH]>,
    msgs: Box<[libc::mmsghdr; BATCH]>,
}

unsafe impl Send for RecvBatch {}

impl Default for RecvBatch {
    fn default() -> Self {
        Self::new()
    }
}

impl RecvBatch {
    pub fn new() -> Self {
        RecvBatch {
            bufs: Box::new([[0u8; MAX_PACKET_SIZE]; BATCH]),
            addrs: Box::new(unsafe { std::mem::zeroed() }),
            iovecs: Box::new(unsafe { std::mem::zeroed() }),
            msgs: Box::new(unsafe { std::mem::zeroed() }),
        }
    }

    pub fn recv(&mut self, fd: RawFd) -> std::io::Result<usize> {
        for i in 0..BATCH {
            let buf_ptr = self.bufs[i].as_mut_ptr() as *mut libc::c_void;
            let addr_ptr = (&mut self.addrs[i] as *mut libc::sockaddr_storage) as *mut libc::c_void;
            self.iovecs[i].iov_base = buf_ptr;
            self.iovecs[i].iov_len = MAX_PACKET_SIZE;
            let iov_ptr = &mut self.iovecs[i] as *mut libc::iovec;
            let hdr = &mut self.msgs[i].msg_hdr;
            hdr.msg_name = addr_ptr;
            hdr.msg_namelen = std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;
            hdr.msg_iov = iov_ptr;
            hdr.msg_iovlen = 1;
            hdr.msg_control = std::ptr::null_mut();
            hdr.msg_controllen = 0;
            hdr.msg_flags = 0;
            self.msgs[i].msg_len = 0;
        }
        let ret = unsafe {
            libc::recvmmsg(
                fd,
                self.msgs.as_mut_ptr(),
                BATCH as libc::c_uint,
                libc::MSG_DONTWAIT as _,
                std::ptr::null_mut(),
            )
        };
        if ret < 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(ret as usize)
    }

    pub fn datagram(&self, index: usize) -> Option<(&[u8], SocketAddr)> {
        let len = (self.msgs[index].msg_len as usize).min(MAX_PACKET_SIZE);
        let buf = &self.bufs[index][..len];
        let addr = sockaddr_to_socketaddr(&self.addrs[index])?;
        Some((buf, addr))
    }
}

pub(crate) fn sockaddr_to_socketaddr(storage: &libc::sockaddr_storage) -> Option<SocketAddr> {
    match storage.ss_family as libc::c_int {
        libc::AF_INET => {
            let addr = unsafe { &*(storage as *const libc::sockaddr_storage as *const libc::sockaddr_in) };
            let ip = Ipv4Addr::from(u32::from_be(addr.sin_addr.s_addr));
            let port = u16::from_be(addr.sin_port);
            Some(SocketAddr::new(IpAddr::V4(ip), port))
        }
        libc::AF_INET6 => {
            let addr = unsafe { &*(storage as *const libc::sockaddr_storage as *const libc::sockaddr_in6) };
            let ip = Ipv6Addr::from(addr.sin6_addr.s6_addr);
            let port = u16::from_be(addr.sin6_port);
            Some(SocketAddr::new(IpAddr::V6(ip), port))
        }
        _ => None,
    }
}