use memmap::Mmap;
use std::io::{Cursor, Seek, SeekFrom, Write};

use crate::errors::Result;
use crate::objects::Type;
use crate::pack::read::packfile_read;
use crate::pack::Packfile;
use crate::stores::{Queryable, StorageSet};

pub struct Reader {
    mmap: Mmap,
}

impl Reader {
    pub fn new(mmap: Mmap) -> Self {
        Reader { mmap }
    }
}

impl Packfile for Reader {
    fn read_bounds<W: Write, S: Queryable>(
        &self,
        start: u64,
        end: u64,
        output: &mut W,
        backends: &StorageSet<S>,
    ) -> Result<Type> {
        let mut cursor = Cursor::new(&self.mmap[..end as usize]);
        cursor.seek(SeekFrom::Start(start))?;

        let packfile_type = packfile_read(&mut cursor, output, &mut 0)?;
        let obj_type = packfile_type.decompress(start, &mut cursor, output, Some(backends))?;

        Ok(obj_type)
    }
}

#[cfg(test)]
mod tests {}
