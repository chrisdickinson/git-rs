use std::ops::Range;
use std::io::Cursor;
use memmap::Mmap;

use crate::delta::{ DeltaDecoder, DeltaDecoderStream, OFS_DELTA, REF_DELTA };
use crate::pack::internal_type::PackfileType;
use crate::pack::generic_read::packfile_read;
use crate::errors::{ Result, ErrorKind };
use crate::pack::iter::PackfileIterator;
use crate::stores::StorageSet;
use crate::packindex::Index;
use crate::pack::Packfile;
use crate::objects::Type;
use crate::id::Id;

pub struct Store {
    mmap: Mmap
}

impl Store {
    pub fn new(mmap: Mmap) -> Self {
        Store {
            mmap
        }
    }
}

impl Packfile for Store {
    fn read_bounds(&self, start: u64, end: u64, backends: &StorageSet) -> Result<(u8, Box<std::io::Read>)> {
        let mut cursor = std::io::Cursor::new(&self.mmap[start as usize .. end as usize]);
        let mut output = Vec::new();
        let packfile_type = packfile_read(&mut cursor, &mut output, backends)?;
        match packfile_type {
            PackfileType::Plain(t) => {
                Ok((t, Box::new(Cursor::new(output))))
            },

            PackfileType::OffsetDelta((offset, instructions)) => {
                let (t, mut base_stream) = self.read_bounds(start - offset, start, backends)?;
                let mut base_buf = Vec::new();
                base_stream.read_to_end(&mut base_buf)?;
                let delta_decoder = DeltaDecoder::new(&instructions, base_buf)?;
                let stream: DeltaDecoderStream = delta_decoder.into();
                Ok((t, Box::new(stream)))
            },

            PackfileType::RefDelta((id, instructions)) => {
                let (t, mut base_stream) = match backends.get(&id)? {
                    Some((xs, stream)) => match xs {
                        Type::Commit => (1, stream),
                        Type::Tree => (2, stream),
                        Type::Blob => (3, stream),
                        Type::Tag => (4, stream)
                    },
                    None => return Err(ErrorKind::CorruptedPackfile.into())
                };

                let mut base_buf = Vec::new();
                base_stream.read_to_end(&mut base_buf)?;
                let delta_decoder = DeltaDecoder::new(&instructions, base_buf)?;
                let stream: DeltaDecoderStream = delta_decoder.into();
                Ok((t, Box::new(stream)))
            }
        }
    }

    fn entries(self) -> Result<PackfileIterator> {
        Err(ErrorKind::NotImplemented.into())
    }
}

#[cfg(test)]
mod tests {
}
