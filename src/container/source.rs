//! Where a document's bytes come from. A [`Document`](super::Document) never asks for more than it needs — the header, then the entries a caller touches — so the same code reads a 40-byte ping in memory and a 4 GB recording on disk. In-memory sources also lend slices, so a [`crate::types::BitPackedView`] over them copies nothing.

use crate::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceError {
    /// `offset..offset+len` is not inside the source's `size` bytes.
    OutOfRange { offset: u64, len: u64, size: u64 },
    /// The underlying reader failed.
    Io(String),
}

impl core::fmt::Display for SourceError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SourceError::OutOfRange { offset, len, size } => write!(f, "read {len} bytes at {offset} outside a {size}-byte source"),
            SourceError::Io(s) => write!(f, "io: {s}"),
        }
    }
}

/// Random access to bytes. `read_at` fills `buf` entirely or errors; a short read is an error, never a silent zero-fill.
pub trait ByteSource {
    fn len(&self) -> u64;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), SourceError>;
    /// A borrowed slice when the source is in memory — `None` for a file. Callers that want zero-copy ask here first and fall back to `read_at`.
    fn slice(&self, _offset: u64, _len: u64) -> Option<&[u8]> {
        None
    }
    /// `len` bytes at `offset`, owned.
    fn read_vec(&self, offset: u64, len: u64) -> Result<Vec<u8>, SourceError> {
        if let Some(s) = self.slice(offset, len) {
            return Ok(s.to_vec());
        }
        let n = usize::try_from(len).map_err(|_| SourceError::OutOfRange { offset, len, size: self.len() })?;
        let mut v = vec![0u8; n];
        self.read_at(offset, &mut v)?;
        Ok(v)
    }
}

fn range_of(size: u64, offset: u64, len: u64) -> Result<(usize, usize), SourceError> {
    // WHY/PROOF: offset and len are header offsets read from the file (or a caller's request) — their sum can overflow u64 and wrap under size without this.
    let end = offset.checked_add(len).filter(|&e| e <= size).ok_or(SourceError::OutOfRange { offset, len, size })?;
    match (usize::try_from(offset), usize::try_from(end)) {
        (Ok(a), Ok(b)) => Ok((a, b)),
        _ => Err(SourceError::OutOfRange { offset, len, size }),
    }
}

impl ByteSource for [u8] {
    fn len(&self) -> u64 {
        <[u8]>::len(self) as u64
    }
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), SourceError> {
        let (a, b) = range_of(<[u8]>::len(self) as u64, offset, buf.len() as u64)?;
        buf.copy_from_slice(&self[a..b]);
        Ok(())
    }
    fn slice(&self, offset: u64, len: u64) -> Option<&[u8]> {
        range_of(<[u8]>::len(self) as u64, offset, len).ok().map(|(a, b)| &self[a..b])
    }
}

impl ByteSource for Vec<u8> {
    fn len(&self) -> u64 {
        self.as_slice().len() as u64
    }
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), SourceError> {
        self.as_slice().read_at(offset, buf)
    }
    fn slice(&self, offset: u64, len: u64) -> Option<&[u8]> {
        ByteSource::slice(self.as_slice(), offset, len)
    }
}

impl<T: ByteSource + ?Sized> ByteSource for &T {
    fn len(&self) -> u64 {
        (**self).len()
    }
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), SourceError> {
        (**self).read_at(offset, buf)
    }
    fn slice(&self, offset: u64, len: u64) -> Option<&[u8]> {
        (**self).slice(offset, len)
    }
}

/// A file, read positionally — no shared seek cursor, so a `Document` is safe to read from several threads.
#[cfg(all(feature = "std", unix))]
impl ByteSource for std::fs::File {
    fn len(&self) -> u64 {
        self.metadata().map(|m| m.len()).unwrap_or(0)
    }
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), SourceError> {
        use std::os::unix::fs::FileExt;
        FileExt::read_exact_at(self, buf, offset).map_err(|e| SourceError::Io(e.to_string()))
    }
}

#[cfg(all(feature = "std", windows))]
impl ByteSource for std::fs::File {
    fn len(&self) -> u64 {
        self.metadata().map(|m| m.len()).unwrap_or(0)
    }
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), SourceError> {
        use std::os::windows::fs::FileExt;
        let mut done = 0usize;
        while done < buf.len() {
            let n = FileExt::seek_read(self, &mut buf[done..], offset + done as u64).map_err(|e| SourceError::Io(e.to_string()))?;
            if n == 0 {
                return Err(SourceError::OutOfRange { offset, len: buf.len() as u64, size: self.len() });
            }
            done += n;
        }
        Ok(())
    }
}
