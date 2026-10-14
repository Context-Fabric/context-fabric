//! Bounds-checked little-endian reads from a mapped `.cfr` file.

use std::fs::File;
use std::ops::Range;
use std::path::Path;

use memmap2::Mmap;

use crate::error::{CfError, Result};
use crate::mapped::MappedNodeValue;

pub(crate) fn map_file(path: &Path) -> Result<Mmap> {
    let file = File::open(path).map_err(|source| CfError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    // SAFETY: `Mmap::map` is unsafe because the mapped bytes can change if the
    // file is modified while mapped. Context-Fabric never modifies a `.cfr` in
    // place: `write_compiled` stages a new file and atomically renames it over
    // the destination, so live mappings keep the old inode's bytes unchanged.
    // An external tool rewriting the file in place would break the `&[u8]`
    // immutability contract; that is outside our control (as for every mmap
    // user). Every read goes through bounds-checked accessors that copy into
    // owned integers (no transmutes, aligned casts, or `from_raw_parts`), so
    // the practical outcome is an "invalid compiled corpus" error or stale
    // values, and a truncating writer can raise SIGBUS.
    Ok(unsafe {
        Mmap::map(&file).map_err(|source| CfError::Io {
            path: path.to_path_buf(),
            source,
        })?
    })
}

pub(crate) fn read_string_range_at(
    path: &Path,
    bytes: &[u8],
    offset: &mut usize,
) -> Result<Range<usize>> {
    let len = read_u32_at(path, bytes, offset)? as usize;
    let start = *offset;
    let end = start
        .checked_add(len)
        .ok_or_else(|| invalid_compiled(path, "compiled string offset overflow"))?;
    if end > bytes.len() {
        return Err(invalid_compiled(
            path,
            "unexpected end of compiled corpus string",
        ));
    }
    *offset = end;
    Ok(start..end)
}

pub(crate) fn read_u32_at(path: &Path, bytes: &[u8], offset: &mut usize) -> Result<u32> {
    let value = read_u32_from(path, bytes, *offset)?;
    *offset = offset
        .checked_add(4)
        .ok_or_else(|| invalid_compiled(path, "compiled u32 offset overflow"))?;
    Ok(value)
}

pub(crate) fn read_u32_from(path: &Path, bytes: &[u8], offset: usize) -> Result<u32> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| invalid_compiled(path, "compiled u32 offset overflow"))?;
    let Some(raw) = bytes.get(offset..end) else {
        return Err(invalid_compiled(
            path,
            "unexpected end of compiled corpus u32",
        ));
    };
    let mut value = [0_u8; 4];
    value.copy_from_slice(raw);
    Ok(u32::from_le_bytes(value))
}

fn read_i64_from(path: &Path, bytes: &[u8], offset: usize) -> Result<i64> {
    let end = offset
        .checked_add(8)
        .ok_or_else(|| invalid_compiled(path, "compiled i64 offset overflow"))?;
    let Some(raw) = bytes.get(offset..end) else {
        return Err(invalid_compiled(
            path,
            "unexpected end of compiled corpus i64",
        ));
    };
    let mut value = [0_u8; 8];
    value.copy_from_slice(raw);
    Ok(i64::from_le_bytes(value))
}

/// Reads one CSR row (1-based `index`) from a section starting at `start`
/// (its `row_count` field). Layout: `row_count`, length-prefixed offsets vec
/// (`row_count + 1` entries), length-prefixed data vec.
pub(crate) fn read_csr_row(
    path: &Path,
    bytes: &[u8],
    start: usize,
    index: u32,
) -> Result<Option<Vec<u32>>> {
    Ok(csr_row(path, bytes, start, index)?.map(Iterator::collect))
}

/// Like [`read_csr_row`] but borrows the row from the map: the row's bounds
/// are checked once, then its values are decoded lazily.
pub(crate) fn csr_row<'a>(
    path: &Path,
    bytes: &'a [u8],
    start: usize,
    index: u32,
) -> Result<Option<MappedU32s<'a>>> {
    let row_count = read_u32_from(path, bytes, start)? as usize;
    if index == 0 || index as usize > row_count {
        return Ok(None);
    }
    let offsets_len = read_u32_from(path, bytes, start + 4)? as usize;
    let offsets_base = start + 8;
    let begin = read_u32_from(path, bytes, offsets_base + ((index as usize - 1) * 4))? as usize;
    let end = read_u32_from(path, bytes, offsets_base + (index as usize * 4))? as usize;
    let data_base = offsets_base + (offsets_len * 4) + 4;
    let row = begin
        .checked_mul(4)
        .zip(end.checked_mul(4))
        .and_then(|(from, to)| bytes.get(data_base + from..data_base + to))
        .ok_or_else(|| invalid_compiled(path, "compiled CSR row out of bounds"))?;
    Ok(Some(MappedU32s::new(row)))
}

/// Little-endian u32 values of a bounds-checked byte range of the map.
pub struct MappedU32s<'a> {
    chunks: std::slice::ChunksExact<'a, u8>,
}

impl<'a> MappedU32s<'a> {
    /// Iterates the little-endian u32s of `bytes` (whose length is a multiple of 4).
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self {
            chunks: bytes.chunks_exact(4),
        }
    }
}

impl Iterator for MappedU32s<'_> {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        self.chunks
            .next()
            .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.chunks.size_hint()
    }
}

impl ExactSizeIterator for MappedU32s<'_> {}

pub(crate) fn read_mixed_node_value_from<'a>(
    path: &Path,
    bytes: &'a [u8],
    offset: usize,
) -> Result<MappedNodeValue<'a>> {
    let tag = *bytes
        .get(offset)
        .ok_or_else(|| invalid_compiled(path, "unexpected end of mixed feature value tag"))?;
    match tag {
        0 => {
            let mut string_offset = offset + 1;
            let range = read_string_range_at(path, bytes, &mut string_offset)?;
            std::str::from_utf8(&bytes[range])
                .map(MappedNodeValue::Str)
                .map_err(|_| invalid_compiled(path, "invalid utf-8 string"))
        }
        1 => read_i64_from(path, bytes, offset + 1).map(MappedNodeValue::Int),
        _ => Err(invalid_compiled(path, "invalid feature value tag")),
    }
}

pub(crate) fn skip_mixed_value(path: &Path, bytes: &[u8], offset: usize) -> Result<usize> {
    let tag = *bytes
        .get(offset)
        .ok_or_else(|| invalid_compiled(path, "unexpected end of mixed feature value tag"))?;
    match tag {
        0 => {
            let mut string_offset = offset + 1;
            read_string_range_at(path, bytes, &mut string_offset)?;
            Ok(string_offset)
        }
        1 => offset
            .checked_add(9)
            .ok_or_else(|| invalid_compiled(path, "mixed int row offset overflow")),
        _ => Err(invalid_compiled(path, "invalid feature value tag")),
    }
}

/// Caps a capacity hint read from an untrusted count field at the number of
/// `min_item_bytes`-sized items that can still fit in `remaining_bytes`.
///
/// A corrupted count (e.g. `u32::MAX`) would otherwise request a huge up-front
/// allocation, and an allocation failure aborts the whole interpreter rather
/// than returning an error. With the cap, the read loop runs off the end of the
/// file and fails with a clean "unexpected end of compiled corpus" error.
pub(crate) fn bounded_capacity(
    declared: usize,
    remaining_bytes: usize,
    min_item_bytes: usize,
) -> usize {
    declared.min(remaining_bytes / min_item_bytes.max(1))
}

pub(crate) fn invalid_compiled(path: &Path, message: impl Into<String>) -> CfError {
    CfError::Parse {
        path: path.to_path_buf(),
        line: 0,
        message: message.into(),
    }
}
