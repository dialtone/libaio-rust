#![allow(dead_code)]
use libc::{c_long, c_int, size_t};
pub use libc::timespec;
use std::mem::zeroed;
use std::default::Default;

// Taken from linux/include/uabi/linux/aio_abi.h
// This is a kernel ABI, so there should be no need to worry about it changing.
#[repr(C)]
#[allow(non_camel_case_types)]
pub struct Struct_iocb {
    pub data: u64,             // ends up in io_event.data

    pub key: u32,              // padding, this should be flipped in BE
    pub aio_reserved1: u32,

    pub aio_lio_opcode: u16,
    pub aio_reqprio: i16,
    pub aio_fildes: u32,

    // PREAD/PWRITE -> void *
    // PREADV/PWRITEV -> iovec
    pub aio_buf: u64,
    pub aio_count: u64,        // bytes or iovec entries
    pub aio_offset: i64,

    pub aio_reserved2: u64,

    pub aio_flags: u32,
    pub aio_resfd: u32,
}

impl Default for Struct_iocb {
    fn default() -> Struct_iocb {
        Struct_iocb { aio_lio_opcode: Iocmd::IoCmdNoop as u16,
                      aio_fildes: (-1_i32) as u32,
                      .. unsafe { zeroed() }
        }
    }
}

#[repr(C)]
pub enum Iocmd {
    IoCmdPread = 0,
    IoCmdPwrite = 1,
    IoCmdFsync = 2,
    IoCmdFdsync = 3,
    // IOCB_CMD_PREADX = 4,
    // IOCB_CMD_POLL = 5,
    IoCmdNoop = 6,
    IoCmdPreadv = 7,
    IoCmdPwritev = 8,
}

pub const IOCB_FLAG_RESFD : u32 = 1 << 0;

#[repr(C)]
#[allow(non_camel_case_types)]
pub struct Struct_io_event {
    pub data: u64,
    pub obj: u64,
    pub res: i64,
    pub res2: i64,
}

impl Default for Struct_io_event {
    fn default() -> Struct_io_event {
        unsafe { zeroed() }
    }
}

#[allow(non_camel_case_types)]
pub enum Struct_io_context { }
#[allow(non_camel_case_types)]
pub type io_context_t = *mut Struct_io_context;

//unsafe impl Send for *mut Struct_io_context {}
//unsafe impl Send for *mut Struct_iocb {}

#[repr(C)]
pub struct Struct_iovec {
    pub iov_base: *mut u8,
    pub iov_len: size_t,
}

#[link(name = "aio")]
extern "C" {
    pub fn io_queue_init(maxevents: c_int, ctxp: *mut io_context_t) -> c_int;
    pub fn io_queue_release(ctx: io_context_t) -> c_int;
    pub fn io_queue_run(ctx: io_context_t) -> c_int;
    pub fn io_setup(maxevents: c_int, ctxp: *mut io_context_t) -> c_int;
    pub fn io_destroy(ctx: io_context_t) -> c_int;
    pub fn io_submit(ctx: io_context_t, nr: c_long, ios: *mut *mut Struct_iocb) -> c_int;
    pub fn io_cancel(ctx: io_context_t, iocb: *mut Struct_iocb, evt: *mut Struct_io_event) -> c_int;
    pub fn io_getevents(ctx_id: io_context_t, min_nr: c_long,
                        nr: c_long, events: *mut Struct_io_event,
                        timeout: *mut timespec) -> c_int;
}

#[cfg(test)]
mod test {
    use std::mem::size_of;

    #[test]
    fn test_sizes() {
        // Check against kernel ABI
        assert!(size_of::<super::Struct_io_event>() == 32);
        assert!(size_of::<super::Struct_iocb>() == 64);
    }
}
