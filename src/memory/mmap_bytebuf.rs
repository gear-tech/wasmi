//! An implementation of a `ByteBuf` based on virtual memory.
//!
//! This implementation uses `mmap` on POSIX systems (and should use `VirtualAlloc` on windows).
//! There are possibilities to improve the performance for the reallocating case by reserving
//! memory up to maximum. This might be a problem for systems that don't have a lot of virtual
//! memory (i.e. 32-bit platforms).

use region::Protection;
use std::slice;

struct Mmap(region::Allocation);

impl Mmap {
    /// Create a new mmap mapping
    ///
    /// Returns `Err` if:
    /// - `len` should not exceed `isize::max_value()`
    /// - `len` should be greater than 0.
    /// - `mmap` returns an error (almost certainly means out of memory).
    fn new(len: usize) -> Result<Self, String> {
        if len > isize::max_value() as usize {
            return Err("`len` should not exceed `isize::max_value()`".into());
        }
        if len == 0 {
            return Err("`len` should be greater than 0".into());
        }

        region::alloc(len, Protection::READ_WRITE)
            .map(Self)
            .map_err(|err| err.to_string())
    }

    fn as_slice(&self) -> &[u8] {
        unsafe {
            // Safety Proof:
            // - Aliasing guarantees of `self.ptr` are not violated since `self` is the only owner.
            // - This pointer was allocated for `self.len` bytes and thus is a valid slice.
            // - `self.len` doesn't change throughout the lifetime of `self`.
            // - The value is returned valid for the duration of lifetime of `self`.
            //   `self` cannot be destroyed while the returned slice is alive.
            // - `self.ptr` is of `NonNull` type and thus `.as_ptr()` can never return NULL.
            // - `self.len` cannot be larger than `isize::max_value()`.
            slice::from_raw_parts(self.0.as_ptr(), self.0.len())
        }
    }

    fn as_slice_mut(&mut self) -> &mut [u8] {
        unsafe {
            // Safety Proof:
            // - See the proof for `Self::as_slice`
            // - Additionally, it is not possible to obtain two mutable references for `self.ptr`
            slice::from_raw_parts_mut(self.0.as_mut_ptr(), self.0.len())
        }
    }
}

impl Drop for Mmap {
    fn drop(&mut self) {}
}

pub struct ByteBuf {
    mmap: Option<Mmap>,
}

impl ByteBuf {
    pub fn new(len: usize) -> Result<Self, String> {
        let mmap = if len == 0 {
            None
        } else {
            Some(Mmap::new(len)?)
        };
        Ok(Self { mmap })
    }

    pub fn realloc(&mut self, new_len: usize) -> Result<(), String> {
        let new_mmap = if new_len == 0 {
            None
        } else {
            let mut new_mmap = Mmap::new(new_len)?;
            if let Some(cur_mmap) = self.mmap.take() {
                let src = cur_mmap.as_slice();
                let dst = new_mmap.as_slice_mut();
                let amount = src.len().min(dst.len());
                dst[..amount].copy_from_slice(&src[..amount]);
            }
            Some(new_mmap)
        };

        self.mmap = new_mmap;
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.mmap.as_ref().map(|m| m.0.len()).unwrap_or(0)
    }

    pub fn as_slice(&self) -> &[u8] {
        self.mmap.as_ref().map(|m| m.as_slice()).unwrap_or(&[])
    }

    pub fn as_slice_mut(&mut self) -> &mut [u8] {
        self.mmap
            .as_mut()
            .map(|m| m.as_slice_mut())
            .unwrap_or(&mut [])
    }

    pub fn erase(&mut self) -> Result<(), String> {
        let len = self.len();
        if len > 0 {
            // The order is important.
            //
            // 1. First we clear, and thus drop, the current mmap if any.
            // 2. And then we create a new one.
            //
            // Otherwise we double the peak memory consumption.
            self.mmap = None;
            self.mmap = Some(Mmap::new(len)?);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::ByteBuf;

    const PAGE_SIZE: usize = 4096;

    // This is not required since wasm memories can only grow but nice to have.
    #[test]
    fn byte_buf_shrink() {
        let mut byte_buf = ByteBuf::new(PAGE_SIZE * 3).unwrap();
        byte_buf.realloc(PAGE_SIZE * 2).unwrap();
    }
}
