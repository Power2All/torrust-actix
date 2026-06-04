#![allow(unsafe_op_in_unsafe_fn)]

use crate::udp::structs::parse_pool::ParsePool;
use crate::udp::enums::udp_reply::UdpReply;
use crate::udp::structs::udp_packet::{
    UdpPacket,
    INLINE_PACKET_SIZE
};
use crate::udp::udp::MAX_PACKET_SIZE;
use crossbeam::queue::ArrayQueue;
use log::{
    debug,
    error,
    info
};
use smallvec::SmallVec;
use std::net::{
    Ipv4Addr,
    Ipv6Addr,
    SocketAddr,
    SocketAddrV4,
    SocketAddrV6
};
use std::sync::Arc;
use windows_sys::core::{
    GUID,
    PCSTR
};
use windows_sys::Win32::Foundation::{
    CloseHandle,
    HANDLE
};
use windows_sys::Win32::Networking::WinSock::{
    bind,
    closesocket,
    htons,
    setsockopt,
    WSAGetLastError,
    WSAIoctl,
    WSASocketW,
    WSAStartup,
    ADDRESS_FAMILY,
    AF_INET,
    AF_INET6,
    IN6_ADDR,
    IN6_ADDR_0,
    INVALID_SOCKET,
    IN_ADDR,
    IN_ADDR_0,
    IPPROTO_UDP,
    RIORESULT,
    RIO_BUF,
    RIO_BUFFERID,
    RIO_CORRUPT_CQ,
    RIO_CQ,
    RIO_EVENT_COMPLETION,
    RIO_EXTENSION_FUNCTION_TABLE,
    RIO_NOTIFICATION_COMPLETION,
    RIO_NOTIFICATION_COMPLETION_0,
    RIO_NOTIFICATION_COMPLETION_0_0,
    RIO_RQ,
    SIO_GET_MULTIPLE_EXTENSION_FUNCTION_POINTER,
    SOCKADDR,
    SOCKADDR_IN,
    SOCKADDR_IN6,
    SOCKET,
    SOCK_DGRAM,
    SOL_SOCKET,
    SO_RCVBUF,
    SO_REUSEADDR,
    SO_SNDBUF,
    WSADATA,
    WSA_FLAG_OVERLAPPED,
    WSA_FLAG_REGISTERED_IO
};
use windows_sys::Win32::System::Threading::{
    CreateEventW,
    SetEvent,
    WaitForMultipleObjects
};

const WSAID_MULTIPLE_RIO: GUID = GUID {
    data1: 0x8509_e081,
    data2: 0x96dd,
    data3: 0x4005,
    data4: [0xb1, 0x65, 0x9e, 0x2e, 0xe8, 0xc7, 0x9e, 0x3f],
};

const RIO_INVALID_BUFFERID: RIO_BUFFERID = 0xFFFF_FFFF;
const RECV_SLOTS: usize = 256;
const SEND_SLOTS: usize = 256;
const ADDR_SIZE: usize = 32;
const WAIT_TIMEOUT_MS: u32 = 250;
const SEND_QUEUE_CAPACITY: usize = 8192;

#[derive(Debug)]
pub struct RioSender {
    queue: ArrayQueue<(SocketAddr, SmallVec<[u8; INLINE_PACKET_SIZE]>)>,
    wake: WakeHandle,
}

impl RioSender {
    pub fn send(&self, remote_addr: SocketAddr, payload: &[u8]) {
        if self.queue.push((remote_addr, SmallVec::from_slice(payload))).is_ok() {
            unsafe { SetEvent(self.wake.0); }
        } else {
            debug!("[UDP RIO] send queue full, dropping reply");
        }
    }
}

#[derive(Debug)]
struct WakeHandle(HANDLE);
unsafe impl Send for WakeHandle {}
unsafe impl Sync for WakeHandle {}

pub fn is_available() -> bool {
    unsafe {
        let mut wsadata: WSADATA = std::mem::zeroed();
        if WSAStartup(0x0202, &mut wsadata) != 0 {
            return false;
        }
        let socket = WSASocketW(
            AF_INET as i32,
            SOCK_DGRAM,
            IPPROTO_UDP,
            std::ptr::null(),
            0,
            WSA_FLAG_OVERLAPPED | WSA_FLAG_REGISTERED_IO,
        );
        if socket == INVALID_SOCKET {
            return false;
        }
        let table = load_rio_table(socket);
        closesocket(socket);
        table.is_some()
    }
}

unsafe fn load_rio_table(socket: SOCKET) -> Option<RIO_EXTENSION_FUNCTION_TABLE> {
    let mut table: RIO_EXTENSION_FUNCTION_TABLE = std::mem::zeroed();
    let mut guid = WSAID_MULTIPLE_RIO;
    let mut bytes_returned: u32 = 0;
    let rc = WSAIoctl(
        socket,
        SIO_GET_MULTIPLE_EXTENSION_FUNCTION_POINTER,
        &mut guid as *mut GUID as *const core::ffi::c_void,
        std::mem::size_of::<GUID>() as u32,
        &mut table as *mut RIO_EXTENSION_FUNCTION_TABLE as *mut core::ffi::c_void,
        std::mem::size_of::<RIO_EXTENSION_FUNCTION_TABLE>() as u32,
        &mut bytes_returned,
        std::ptr::null_mut(),
        None,
    );
    if rc == 0 { Some(table) } else { None }
}

struct RegisteredBuffer {
    bytes: Vec<u8>,
    id: RIO_BUFFERID,
}

struct RioContext {
    socket: SOCKET,
    table: RIO_EXTENSION_FUNCTION_TABLE,
    cq: RIO_CQ,
    rq: RIO_RQ,
    recv_data: RegisteredBuffer,
    recv_addr: RegisteredBuffer,
    send_data: RegisteredBuffer,
    send_addr: RegisteredBuffer,
}

impl Drop for RioContext {
    fn drop(&mut self) {
        unsafe {
            if let Some(dereg) = self.table.RIODeregisterBuffer {
                dereg(self.recv_data.id);
                dereg(self.recv_addr.id);
                dereg(self.send_data.id);
                dereg(self.send_addr.id);
            }
            if let Some(close_cq) = self.table.RIOCloseCompletionQueue {
                close_cq(self.cq);
            }
            closesocket(self.socket);
        }
    }
}

pub fn run(
    bind_address: SocketAddr,
    recv_buffer_size: usize,
    send_buffer_size: usize,
    reuse_address: bool,
    parse_pool: Arc<ParsePool>,
    rx: tokio::sync::watch::Receiver<bool>,
) {
    let send_wake = unsafe { CreateEventW(std::ptr::null(), 0, 0, std::ptr::null()) };
    let cq_event = unsafe { CreateEventW(std::ptr::null(), 0, 0, std::ptr::null()) };
    if send_wake.is_null() || cq_event.is_null() {
        error!("[UDP RIO] failed to create wait events; receiver not started");
        return;
    }

    let mut ctx = match unsafe { setup(bind_address, recv_buffer_size, send_buffer_size, reuse_address, cq_event) } {
        Ok(ctx) => ctx,
        Err(e) => {
            error!("[UDP RIO] setup failed for {bind_address}: {e}; receiver not started");
            unsafe {
                CloseHandle(cq_event);
                CloseHandle(send_wake);
            }
            return;
        }
    };

    let sender = Arc::new(RioSender {
        queue: ArrayQueue::new(SEND_QUEUE_CAPACITY),
        wake: WakeHandle(send_wake),
    });

    let mut send_free: Vec<usize> = (0..SEND_SLOTS).collect();

    for slot in 0..RECV_SLOTS {
        if !unsafe { post_receive(&ctx, slot) } {
            error!("[UDP RIO] initial RIOReceiveEx failed; receiver not started");
            unsafe {
                CloseHandle(cq_event);
                CloseHandle(send_wake);
            }
            return;
        }
    }

    let notify = ctx.table.RIONotify.expect("RIONotify present (table validated)");
    let dequeue = ctx.table.RIODequeueCompletion.expect("RIODequeueCompletion present (table validated)");

    unsafe { notify(ctx.cq); }

    info!("[UDP RIO] receive backend started on {bind_address}");

    let handles = [cq_event, send_wake];
    let mut results: [RIORESULT; 256] = unsafe { std::mem::zeroed() };

    loop {
        unsafe { WaitForMultipleObjects(2, handles.as_ptr(), 0, WAIT_TIMEOUT_MS); }

        if *rx.borrow() {
            break;
        }

        loop {
            let count = unsafe { dequeue(ctx.cq, results.as_mut_ptr(), results.len() as u32) };
            if count == RIO_CORRUPT_CQ {
                error!("[UDP RIO] completion queue corrupt; stopping receiver");
                unsafe {
                    CloseHandle(cq_event);
                    CloseHandle(send_wake);
                }
                return;
            }
            if count == 0 {
                break;
            }
            for result in &results[..count as usize] {
                let request_context = result.RequestContext as usize;
                if request_context < RECV_SLOTS {
                    let slot = request_context;
                    if result.Status == 0 && result.BytesTransferred > 0 {
                        let len = (result.BytesTransferred as usize).min(MAX_PACKET_SIZE);
                        let data_off = slot * MAX_PACKET_SIZE;
                        let addr_off = slot * ADDR_SIZE;
                        if let Some(remote_addr) = unsafe { read_sockaddr(ctx.recv_addr.bytes.as_ptr().add(addr_off)) } {
                            let packet = UdpPacket {
                                remote_addr,
                                data: SmallVec::from_slice(&ctx.recv_data.bytes[data_off..data_off + len]),
                                reply: UdpReply::Rio(sender.clone()),
                            };
                            if parse_pool.payload.push(packet).is_err() {
                                debug!("[UDP RIO] parse pool queue full, dropping packet");
                            }
                        }
                    }
                    if !unsafe { post_receive(&ctx, slot) } {
                        error!("[UDP RIO] RIOReceiveEx re-post failed; stopping receiver");
                        unsafe {
                            CloseHandle(cq_event);
                            CloseHandle(send_wake);
                        }
                        return;
                    }
                } else {
                    let slot = request_context - RECV_SLOTS;
                    if slot < SEND_SLOTS {
                        send_free.push(slot);
                    }
                }
            }
            unsafe { notify(ctx.cq); }
            if (count as usize) < results.len() {
                break;
            }
        }

        while !send_free.is_empty() {
            let Some((addr, payload)) = sender.queue.pop() else { break; };
            let slot = send_free.pop().unwrap();
            if !unsafe { post_send(&mut ctx, slot, addr, &payload) } {
                debug!("[UDP RIO] RIOSendEx failed, dropping reply");
                send_free.push(slot);
            }
        }
    }

    unsafe {
        CloseHandle(cq_event);
        CloseHandle(send_wake);
    }
    info!("Stopping UDP server: {bind_address}...");
}

unsafe fn setup(
    bind_address: SocketAddr,
    recv_buffer_size: usize,
    send_buffer_size: usize,
    reuse_address: bool,
    cq_event: HANDLE,
) -> Result<RioContext, String> {
    let mut wsadata: WSADATA = std::mem::zeroed();
    if WSAStartup(0x0202, &mut wsadata) != 0 {
        return Err("WSAStartup failed".to_string());
    }

    let family = if bind_address.is_ipv4() { AF_INET } else { AF_INET6 };
    let socket = WSASocketW(
        family as i32,
        SOCK_DGRAM,
        IPPROTO_UDP,
        std::ptr::null(),
        0,
        WSA_FLAG_OVERLAPPED | WSA_FLAG_REGISTERED_IO,
    );
    if socket == INVALID_SOCKET {
        return Err(format!("WSASocketW failed (WSA error {})", WSAGetLastError()));
    }

    if reuse_address {
        let opt: i32 = 1;
        setsockopt(socket, SOL_SOCKET, SO_REUSEADDR, &opt as *const i32 as PCSTR, std::mem::size_of::<i32>() as i32);
    }
    if recv_buffer_size > 0 {
        let opt = recv_buffer_size as i32;
        setsockopt(socket, SOL_SOCKET, SO_RCVBUF, &opt as *const i32 as PCSTR, std::mem::size_of::<i32>() as i32);
    }
    if send_buffer_size > 0 {
        let opt = send_buffer_size as i32;
        setsockopt(socket, SOL_SOCKET, SO_SNDBUF, &opt as *const i32 as PCSTR, std::mem::size_of::<i32>() as i32);
    }

    let mut storage = [0u8; ADDR_SIZE];
    let addr_len = write_sockaddr(storage.as_mut_ptr(), &bind_address);
    if bind(socket, storage.as_ptr() as *const SOCKADDR, addr_len as i32) != 0 {
        let err = WSAGetLastError();
        closesocket(socket);
        return Err(format!("bind to {bind_address} failed (WSA error {err})"));
    }

    let table = match load_rio_table(socket) {
        Some(t) => t,
        None => {
            closesocket(socket);
            return Err("RIO function table unavailable".to_string());
        }
    };

    let register = match table.RIORegisterBuffer {
        Some(f) => f,
        None => { closesocket(socket); return Err("RIORegisterBuffer missing".to_string()); }
    };
    let create_cq = match table.RIOCreateCompletionQueue {
        Some(f) => f,
        None => { closesocket(socket); return Err("RIOCreateCompletionQueue missing".to_string()); }
    };
    let create_rq = match table.RIOCreateRequestQueue {
        Some(f) => f,
        None => { closesocket(socket); return Err("RIOCreateRequestQueue missing".to_string()); }
    };

    let recv_data = match register_buffer(register, RECV_SLOTS * MAX_PACKET_SIZE) {
        Ok(b) => b,
        Err(e) => { closesocket(socket); return Err(e); }
    };
    let recv_addr = match register_buffer(register, RECV_SLOTS * ADDR_SIZE) {
        Ok(b) => b,
        Err(e) => { closesocket(socket); return Err(e); }
    };
    let send_data = match register_buffer(register, SEND_SLOTS * MAX_PACKET_SIZE) {
        Ok(b) => b,
        Err(e) => { closesocket(socket); return Err(e); }
    };
    let send_addr = match register_buffer(register, SEND_SLOTS * ADDR_SIZE) {
        Ok(b) => b,
        Err(e) => { closesocket(socket); return Err(e); }
    };

    let notification = event_notification(cq_event);
    let cq_size = (RECV_SLOTS + SEND_SLOTS) as u32;
    let cq = create_cq(cq_size, &notification);
    if cq == 0 {
        let err = WSAGetLastError();
        closesocket(socket);
        return Err(format!("RIOCreateCompletionQueue failed (WSA error {err})"));
    }

    let rq = create_rq(
        socket,
        RECV_SLOTS as u32,
        1,
        SEND_SLOTS as u32,
        1,
        cq,
        cq,
        std::ptr::null(),
    );
    if rq == 0 {
        let err = WSAGetLastError();
        if let Some(close_cq) = table.RIOCloseCompletionQueue { close_cq(cq); }
        closesocket(socket);
        return Err(format!("RIOCreateRequestQueue failed (WSA error {err})"));
    }

    Ok(RioContext { socket, table, cq, rq, recv_data, recv_addr, send_data, send_addr })
}

fn event_notification(event: HANDLE) -> RIO_NOTIFICATION_COMPLETION {
    RIO_NOTIFICATION_COMPLETION {
        Type: RIO_EVENT_COMPLETION,
        Anonymous: RIO_NOTIFICATION_COMPLETION_0 {
            Event: RIO_NOTIFICATION_COMPLETION_0_0 {
                EventHandle: event,
                NotifyReset: 1,
            },
        },
    }
}

type RegisterFn = unsafe extern "system" fn(PCSTR, u32) -> RIO_BUFFERID;

unsafe fn register_buffer(register: RegisterFn, size: usize) -> Result<RegisteredBuffer, String> {
    let bytes = vec![0u8; size];
    let id = register(bytes.as_ptr() as PCSTR, size as u32);
    if id == RIO_INVALID_BUFFERID {
        return Err(format!("RIORegisterBuffer failed (WSA error {})", WSAGetLastError()));
    }
    Ok(RegisteredBuffer { bytes, id })
}

unsafe fn post_receive(ctx: &RioContext, slot: usize) -> bool {
    let receive = match ctx.table.RIOReceiveEx {
        Some(f) => f,
        None => return false,
    };
    let data = RIO_BUF {
        BufferId: ctx.recv_data.id,
        Offset: (slot * MAX_PACKET_SIZE) as u32,
        Length: MAX_PACKET_SIZE as u32,
    };
    let addr = RIO_BUF {
        BufferId: ctx.recv_addr.id,
        Offset: (slot * ADDR_SIZE) as u32,
        Length: ADDR_SIZE as u32,
    };
    let rc = receive(
        ctx.rq,
        &data,
        1,
        std::ptr::null(),
        &addr,
        std::ptr::null(),
        std::ptr::null(),
        0,
        slot as *const core::ffi::c_void,
    );
    rc != 0
}

unsafe fn post_send(ctx: &mut RioContext, slot: usize, remote_addr: SocketAddr, payload: &[u8]) -> bool {
    let send = match ctx.table.RIOSendEx {
        Some(f) => f,
        None => return false,
    };
    let len = payload.len().min(MAX_PACKET_SIZE);
    let data_off = slot * MAX_PACKET_SIZE;
    let addr_off = slot * ADDR_SIZE;
    ctx.send_data.bytes[data_off..data_off + len].copy_from_slice(&payload[..len]);
    let addr_len = write_sockaddr(ctx.send_addr.bytes.as_mut_ptr().add(addr_off), &remote_addr);

    let data = RIO_BUF {
        BufferId: ctx.send_data.id,
        Offset: data_off as u32,
        Length: len as u32,
    };
    let addr = RIO_BUF {
        BufferId: ctx.send_addr.id,
        Offset: addr_off as u32,
        Length: addr_len,
    };
    let rc = send(
        ctx.rq,
        &data,
        1,
        std::ptr::null(),
        &addr,
        std::ptr::null(),
        std::ptr::null(),
        0,
        (RECV_SLOTS + slot) as *const core::ffi::c_void,
    );
    rc != 0
}

unsafe fn write_sockaddr(buf: *mut u8, addr: &SocketAddr) -> u32 {
    match addr {
        SocketAddr::V4(v4) => {
            let sa = buf as *mut SOCKADDR_IN;
            (*sa).sin_family = AF_INET as ADDRESS_FAMILY;
            (*sa).sin_port = htons(v4.port());
            (*sa).sin_addr = IN_ADDR { S_un: IN_ADDR_0 { S_addr: u32::from_ne_bytes(v4.ip().octets()) } };
            std::mem::size_of::<SOCKADDR_IN>() as u32
        }
        SocketAddr::V6(v6) => {
            let sa = buf as *mut SOCKADDR_IN6;
            (*sa).sin6_family = AF_INET6 as ADDRESS_FAMILY;
            (*sa).sin6_port = htons(v6.port());
            (*sa).sin6_flowinfo = v6.flowinfo();
            (*sa).sin6_addr = IN6_ADDR { u: IN6_ADDR_0 { Byte: v6.ip().octets() } };
            (*sa).Anonymous.sin6_scope_id = v6.scope_id();
            std::mem::size_of::<SOCKADDR_IN6>() as u32
        }
    }
}

unsafe fn read_sockaddr(buf: *const u8) -> Option<SocketAddr> {
    let family = (*(buf as *const SOCKADDR)).sa_family;
    if family == AF_INET as ADDRESS_FAMILY {
        let sa = &*(buf as *const SOCKADDR_IN);
        let ip = Ipv4Addr::from(sa.sin_addr.S_un.S_addr.to_ne_bytes());
        let port = u16::from_be(sa.sin_port);
        Some(SocketAddr::V4(SocketAddrV4::new(ip, port)))
    } else if family == AF_INET6 as ADDRESS_FAMILY {
        let sa = &*(buf as *const SOCKADDR_IN6);
        let ip = Ipv6Addr::from(sa.sin6_addr.u.Byte);
        let port = u16::from_be(sa.sin6_port);
        let scope = sa.Anonymous.sin6_scope_id;
        Some(SocketAddr::V6(SocketAddrV6::new(ip, port, sa.sin6_flowinfo, scope)))
    } else {
        None
    }
}