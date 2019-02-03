use std::io::{ Cursor, Write, Seek };
use std::ops::Range;
use memmap::Mmap;

use crate::delta::{ DeltaDecoder, DeltaDecoderStream, OFS_DELTA, REF_DELTA };
use crate::pack::read::packfile_read_decompressed;
use crate::pack::internal_type::PackfileType;
use crate::stores::{ Queryable, StorageSet };
use crate::errors::{ Result, ErrorKind };
use crate::pack::iter::PackfileIterator;
use crate::packindex::Index;
use crate::pack::Packfile;
use crate::objects::Type;
use crate::id::Id;

pub struct Reader {
    mmap: Mmap
}

impl Reader {
    pub fn new(mmap: Mmap) -> Self {
        Reader {
            mmap
        }
    }
}

impl Packfile for Reader {
    fn read_bounds<W: Write, S: Queryable>(&self, start: u64, end: u64, output: &mut W, backends: &StorageSet<S>) -> Result<Type> {
        let mut cursor = std::io::Cursor::new(&self.mmap[ .. end as usize]);
        cursor.seek(std::io::SeekFrom::Start(start))?;

        let (_, obj_type) = packfile_read_decompressed(&mut cursor, output, Some(backends), None)?;
        Ok(obj_type)
    }
}

#[cfg(test)]
mod tests {
}
