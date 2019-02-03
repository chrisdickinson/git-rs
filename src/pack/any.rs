use std::io::{ BufReader, SeekFrom };
use flate2::bufread::DeflateDecoder;
use std::io::prelude::*;
use std::io::{ Cursor, Read, Write, Seek };
use std;

use crate::delta::{ DeltaDecoder, DeltaDecoderStream, OFS_DELTA, REF_DELTA };
use crate::pack::read::packfile_read_decompressed;
use crate::pack::internal_type::PackfileType;
use crate::stores::{ Queryable, StorageSet };
use crate::errors::{ Result, ErrorKind };
use crate::pack::iter::PackfileIterator;
use crate::pack::Packfile;
use crate::objects::Type;
use crate::id::Id;

pub type GetObject = Fn(&Id) -> Result<Option<(Type, Box<std::io::Read>)>>;

pub struct Reader<R> {
    read: Box<Fn() -> Result<R>>,
}

impl<R: Read + Seek + 'static> Reader<R> {
    pub fn new<C>(func: C) -> Self
        where C: Fn() -> Result<R> + 'static {

        Reader {
            read: Box::new(func)
        }
    }
}

impl<R: Read + Seek + std::fmt::Debug> Packfile for Reader<R> {
    fn read_bounds<W: Write, S: Queryable>(
        &self,
        start: u64,
        end: u64,
        output: &mut W,
        backends: &StorageSet<S>
    ) -> Result<Type> {
        let handle = (self.read)()?;
        let mut buffered_file = BufReader::new(handle);
        let mut output = Vec::new();
        buffered_file.seek(SeekFrom::Start(start))?;
        let (_, obj_type) = packfile_read_decompressed(&mut buffered_file, &mut output, Some(backends), None)?;
        Ok(obj_type)
    }
}
