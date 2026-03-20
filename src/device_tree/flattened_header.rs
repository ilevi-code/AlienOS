use super::{bytes_reader::BytesReader, error::FdtParseError, string_block::StringBlock};

#[repr(C)]
pub(super) struct FlattenedHeader {
    magic: u32,
    size: u32,
    struct_offset: u32,
    string_offset: u32,
    _mem_reserve: u32,
    _version: u32,
    _last_compatible: u32,
    _boot_cpu_id: u32,
    strings_size: u32,
    struct_size: u32,
}

impl FlattenedHeader {
    pub(super) fn strings(&self) -> Result<StringBlock<'_>, FdtParseError<'_>> {
        let block = self.slice_at(
            u32::from_be(self.string_offset),
            u32::from_be(self.strings_size),
        )?;
        Ok(StringBlock::new(block))
    }

    pub(super) fn structs(&self) -> Result<BytesReader<'_>, FdtParseError<'_>> {
        let block = self.slice_at(
            u32::from_be(self.struct_offset),
            u32::from_be(self.struct_size),
        )?;
        Ok(BytesReader::new(block))
    }

    pub(crate) fn len(&self) -> usize {
        u32::from_be(self.size) as usize
    }

    fn slice_at<T>(&self, offset: u32, size: u32) -> Result<&[T], FdtParseError<'_>> {
        let addr = (self as *const FlattenedHeader) as *const T;
        let addr = unsafe { addr.byte_add(offset as usize) };
        if u32::from_be(self.size) < offset + size {
            return Err(FdtParseError::CorruptHeader);
        }
        Ok(unsafe { core::slice::from_raw_parts(addr, size as usize) })
    }
}
