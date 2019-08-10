pub use buf::{RdBuf,WrBuf};
use std::os::unix::io::{RawFd, AsRawFd};

mod aioabi;
mod buf;
mod pool;

pub mod raw;
pub mod directio;
pub mod aligned;

/// Wrapper for file offset
pub type Offset = u64;

/// Wrapper for a file descriptor.
struct FD(RawFd);

impl AsRawFd for FD {
    fn as_raw_fd(&self) -> RawFd {
        let FD(fd) = *self;
        fd
    }
}
