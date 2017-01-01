//! Aligned memory buffers for Direct IO.
extern crate aligned_alloc;
use std::ptr;
use std::slice;
use std::mem;

use buf::{RdBuf, WrBuf};

/// Allocate and manage buffers with fixed memory alignment.
///
/// This is intended to be used with Directio, which has such
/// requirements. The buffer has two sizes associated with it: the
/// actual number of allocated bytes, which is always a multiple of
/// the alignment, and the number of valid (initialized) bytes.
pub struct AlignedBuf {
    buf: *mut u8,               // pointer to allocated memory
    align: usize,               // alignment of buffer
    len: usize,                 // length of allocated memory
    valid: usize,               // length of valid/initialized memory
}

unsafe impl Send for AlignedBuf {}

fn ispower2(n: usize) -> bool {
    (n & (n - 1)) == 0
}

impl AlignedBuf {
    /// Allocate some uninitialized memory. No bytes are valid as a
    /// result of this. Returns `None` on allocation failure.
    ///
    /// # Preconditions
    /// `align` must be a power of 2, and greater than 0.
    pub unsafe fn alloc_uninit(size: usize, align: usize) -> Option<AlignedBuf> {
        assert!(align > 0);
        assert!(ispower2(align));

        let sz = (size + align - 1) & !(align - 1);
        assert!(sz >= size);
        assert!(sz % align == 0);
        let p = aligned_alloc::aligned_alloc(sz, align);

        if p.is_null() {
            None
        } else {
            Some(AlignedBuf { buf: mem::transmute(p), len: sz, valid: 0, align: align })
        }
    }

    /// Allocate a buffer initialized to bytes.
    pub fn alloc(size: usize, align: usize) -> Option<AlignedBuf> {
        unsafe {
            match AlignedBuf::alloc_uninit(size, align) {
                None => None,
                Some(mut b) => {
                    ptr::write_bytes(b.buf, 0, b.len);
                    b.valid = b.len;
                    Some(b)
                }
            }
        }
    }

    /// Allocate a buffer and initialize it from a slice.
    pub fn from_slice(data: &[u8], align: usize) -> Option<AlignedBuf> {
        unsafe {
            match AlignedBuf::alloc_uninit(data.len(), align) {
                None => None,
                Some(mut b) => {
                    ptr::copy_nonoverlapping(data.as_ptr(), b.buf, data.len());
                    if data.len() != b.len {
                        assert!(b.len > data.len());
                        ptr::write_bytes((b.buf as usize + data.len()) as *mut u8, 0, b.len - data.len())
                    };
                    b.valid = b.len;
                    Some(b)
                }
            }
        }
    }

    pub fn as_slice(&self) -> &[u8] { self.wrbuf() }
    
    pub unsafe fn as_ptr(&self) -> *const u8 {
        self.buf as *const u8
    }

    pub unsafe fn as_mut_ptr(&mut self) -> *mut u8 {
        self.buf
    }

    pub fn len(&self) -> usize { self.len }
    pub fn valid(&self) -> usize { self.valid }
}

impl Drop for AlignedBuf {
    fn drop(&mut self) {
        unsafe { aligned_alloc::aligned_free(mem::transmute(self.buf)) }
    }
}

impl Clone for AlignedBuf {
    /// Clones the buffer, copying the valid portion of it from the
    /// source. The non-valid part of the result has undefined
    /// contents which may be different from the source.
    fn clone(&self) -> AlignedBuf {
        assert!(self.valid <= self.len);
        unsafe {
            match AlignedBuf::alloc_uninit(self.len, self.align) {
                None => panic!("clone failed"),
                Some(mut b) => {
                    if b.valid > 0 {
                        ptr::copy_nonoverlapping(self.buf as *const u8, b.buf, b.valid);
                        b.valid = self.valid
                    };
                    b
                }
            }
        }
    }
}

impl RdBuf for AlignedBuf {
    /// Return a writable slice to the whole buffer; it may not be
    /// initialized, and so should be treated as write-only.
    fn rdbuf<'a>(&'a mut self) -> &'a mut [u8] {
        assert!(self.valid <= self.len);
        unsafe { slice::from_raw_parts_mut(self.buf, self.len) }
    }

    /// Update the valid portion of the buffer.
    fn rdupdate(&mut self, base: usize, len: usize) {
        assert!(self.valid <= self.len);
        if base <= self.valid && base+len > self.valid {
            assert!(base+len <= self.len);
            self.valid = base+len;
        }
    }
}

impl WrBuf for AlignedBuf {
    /// Return a read-only slice of the valid portion of the buffer.
    fn wrbuf<'a>(&'a self) -> &'a [u8] {
        assert!(self.valid <= self.len);
        unsafe { slice::from_raw_parts_mut(self.buf, self.valid) }
    }
}

#[cfg(test)]
mod test {
    use super::AlignedBuf;

    fn alloc(size: usize, align: usize) -> AlignedBuf {
        match AlignedBuf::alloc(size, align) {
            None => panic!("alloc failed"),
            Some(p) => p,
        }
    }

    #[test]
    fn aligned() {
        let p = alloc(16, 16);
        assert_eq!(p.as_slice().len(), 16);

        let p = alloc(10, 16);
        assert_eq!(p.as_slice().len(), 16);

        let p = alloc(17, 16);
        assert_eq!(p.as_slice().len(), 32);
    }
}
