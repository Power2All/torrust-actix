use crate::udp::enums::udp_reply::UdpReply;
use crate::udp::impls::batch_recv::sockaddr_to_socketaddr;
use crate::udp::structs::parse_pool::ParsePool;
use crate::udp::structs::udp_packet::UdpPacket;
use crate::udp::udp::MAX_PACKET_SIZE;
use io_uring::{
    opcode,
    types,
    IoUring
};
use log::{
    debug,
    error,
    info
};
use smallvec::SmallVec;
use std::os::unix::io::AsRawFd;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;

const IN_FLIGHT: usize = 128;
const RING_ENTRIES: u32 = 256;
const SHUTDOWN_POLL: Duration = Duration::from_millis(250);

/// Probes the running kernel for the `io_uring` features this backend needs.
pub fn is_available() -> bool {
    IoUring::new(8).is_ok()
}

struct Slots {
    bufs: Box<[[u8; MAX_PACKET_SIZE]; IN_FLIGHT]>,
    addrs: Box<[libc::sockaddr_storage; IN_FLIGHT]>,
    iovecs: Box<[libc::iovec; IN_FLIGHT]>,
    msghdrs: Box<[libc::msghdr; IN_FLIGHT]>,
}

impl Slots {
    fn new() -> Self {
        Slots {
            bufs: Box::new([[0u8; MAX_PACKET_SIZE]; IN_FLIGHT]),
            addrs: Box::new(unsafe { std::mem::zeroed() }),
            iovecs: Box::new(unsafe { std::mem::zeroed() }),
            msghdrs: Box::new(unsafe { std::mem::zeroed() }),
        }
    }

    fn prepare(&mut self, i: usize) -> *mut libc::msghdr {
        let buf_ptr = self.bufs[i].as_mut_ptr() as *mut libc::c_void;
        let addr_ptr = (&mut self.addrs[i] as *mut libc::sockaddr_storage) as *mut libc::c_void;
        self.iovecs[i].iov_base = buf_ptr;
        self.iovecs[i].iov_len = MAX_PACKET_SIZE;
        let iov_ptr = &mut self.iovecs[i] as *mut libc::iovec;
        let hdr = &mut self.msghdrs[i];
        hdr.msg_name = addr_ptr;
        hdr.msg_namelen = std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;
        hdr.msg_iov = iov_ptr;
        hdr.msg_iovlen = 1;
        hdr.msg_control = std::ptr::null_mut();
        hdr.msg_controllen = 0;
        hdr.msg_flags = 0;
        &mut self.msghdrs[i] as *mut libc::msghdr
    }
}

/// Runs the `io_uring`-based receive loop on a dedicated thread, pushing datagrams into
/// the parse pool until shutdown.
pub fn run(socket: Arc<UdpSocket>, parse_pool: Arc<ParsePool>, rx: tokio::sync::watch::Receiver<bool>) {
    let fd = socket.as_raw_fd();
    let udp_sock = socket.local_addr().ok();
    let mut ring = match IoUring::new(RING_ENTRIES) {
        Ok(ring) => ring,
        Err(e) => {
            error!("[UDP] io_uring init failed ({e}); receiver not started");
            return;
        }
    };
    let mut slots = Slots::new();

    if let Err(e) = submit_slots(&mut ring, &mut slots, 0..IN_FLIGHT, fd) {
        error!("[UDP] io_uring initial submit failed: {e}; receiver not started");
        return;
    }

    let timespec = types::Timespec::new()
        .sec(SHUTDOWN_POLL.as_secs())
        .nsec(SHUTDOWN_POLL.subsec_nanos());
    let args = types::SubmitArgs::new().timespec(&timespec);

    loop {
        match ring.submitter().submit_with_args(1, &args) {
            Ok(_) => {}
            Err(ref e) if e.raw_os_error() == Some(libc::ETIME) => {
                if *rx.borrow() {
                    break;
                }
                continue;
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => {
                error!("[UDP] io_uring wait failed: {e}; stopping receiver");
                break;
            }
        }

        let mut completed: Vec<(usize, i32)> = Vec::with_capacity(IN_FLIGHT);
        {
            for cqe in ring.completion() {
                completed.push((cqe.user_data() as usize, cqe.result()));
            }
        }

        for &(slot, res) in &completed {
            if slot >= IN_FLIGHT {
                continue;
            }
            if res > 0 {
                let len = (res as usize).min(MAX_PACKET_SIZE);
                if let Some(remote_addr) = sockaddr_to_socketaddr(&slots.addrs[slot]) {
                    let packet = UdpPacket {
                        remote_addr,
                        data: SmallVec::from_slice(&slots.bufs[slot][..len]),
                        reply: UdpReply::Socket(socket.clone()),
                    };
                    if parse_pool.payload.push(packet).is_err() {
                        debug!("Parse pool queue full, dropping packet");
                    }
                }
            }
        }

        let slots_to_resubmit: Vec<usize> = completed.iter().map(|&(slot, _)| slot).filter(|&slot| slot < IN_FLIGHT).collect();
        if !slots_to_resubmit.is_empty()
            && let Err(e) = submit_slots(&mut ring, &mut slots, slots_to_resubmit, fd) {
            error!("[UDP] io_uring resubmit failed: {e}; stopping receiver");
            break;
        }

        if *rx.borrow() {
            break;
        }
    }

    if let Some(addr) = udp_sock {
        info!("Stopping UDP server: {addr}...");
    }
}

fn submit_slots(ring: &mut IoUring, slots: &mut Slots, indices: impl IntoIterator<Item = usize>, fd: std::os::unix::io::RawFd) -> std::io::Result<()> {
    let entries: Vec<io_uring::squeue::Entry> = indices
        .into_iter()
        .map(|slot| {
            let hdr = slots.prepare(slot);
            opcode::RecvMsg::new(types::Fd(fd), hdr).build().user_data(slot as u64)
        })
        .collect();
    {
        let mut sq = ring.submission();
        for entry in &entries {
            if unsafe { sq.push(entry) }.is_err() {
                return Err(std::io::Error::other("io_uring submission queue full"));
            }
        }
    }
    ring.submit()?;
    Ok(())
}