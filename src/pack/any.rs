use std::io::{ BufReader, SeekFrom };
use flate2::bufread::DeflateDecoder;
use std::io::prelude::*;
use std::io::Cursor;
use std;

use crate::delta::{ DeltaDecoder, DeltaDecoderStream, OFS_DELTA, REF_DELTA };
use crate::pack::internal_type::PackfileType;
use crate::pack::generic_read::packfile_read;
use crate::stores::{ Storage, StorageSet };
use crate::errors::{ Result, ErrorKind };
use crate::pack::iter::PackfileIterator;
use crate::pack::Packfile;
use crate::objects::Type;
use crate::id::Id;

pub type GetObject = Fn(&Id) -> Result<Option<(Type, Box<std::io::Read>)>>;

pub struct Reader<R> {
    read: Box<Fn() -> Result<R>>,
}

impl<R: std::io::Read + std::io::Seek + 'static> Reader<R> {
    pub fn new<C>(func: C) -> Self
        where C: Fn() -> Result<R> + 'static {

        Reader {
            read: Box::new(func)
        }
    }
}

impl<R: std::io::Read + std::io::Seek + std::fmt::Debug> Packfile for Reader<R> {
    fn read_bounds (&self, start: u64, end: u64, backends: &StorageSet) -> Result<(u8, Box<std::io::Read>)> {
        let handle = (self.read)()?;
        let mut buffered_file = BufReader::new(handle);
        let mut output = Vec::new();
        buffered_file.seek(SeekFrom::Start(start))?;
        let packfile_type = packfile_read(&mut buffered_file, &mut output)?;
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
}
