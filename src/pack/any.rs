use std::io::{ BufReader, SeekFrom };
use std::io::{ Read, Write, Seek };
use std;

use crate::stores::{ Queryable, StorageSet };
use crate::pack::read::packfile_read;
use crate::errors::Result;
use crate::pack::Packfile;
use crate::objects::Type;
use crate::id::Id;

pub type GetObject = dyn Fn(&Id) -> Result<Option<(Type, Box<dyn std::io::Read>)>>;

pub struct Reader<R> {
    read: Box<dyn Fn() -> Result<R>>,
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
        _end: u64,
        output: &mut W,
        backends: &StorageSet<S>
    ) -> Result<Type> {
        let handle = (self.read)()?;
        let mut buffered_file = BufReader::new(handle);
        buffered_file.seek(SeekFrom::Start(start))?;

        let packfile_type = packfile_read(&mut buffered_file, output, &mut 0)?;
        let obj_type = packfile_type.decompress(
            start,
            &mut buffered_file,
            output,
            Some(backends)
        )?;
        Ok(obj_type)
    }
}
