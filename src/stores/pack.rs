use std::io::{ Cursor, Write };

use crate::stores::{ Queryable, StorageSet };
use crate::errors::{ Result, ErrorKind };
use crate::packindex::Index;
use crate::pack::Packfile;
use crate::objects::Type;
use crate::id::Id;

pub struct Store<P: Packfile> {
    packfile: P,
    index: Index
}

impl<P: Packfile> Store<P> {
    pub fn new (packfile: P, index: Index) -> Self {
        Store {
            packfile,
            index
        }
    }
}

impl<P: Packfile> Queryable for Store<P> {
    fn get<W: Write, S: Queryable>(&self, id: &Id, output: &mut W, backends: &StorageSet<S>) -> Result<Option<Type>> {
        let (start, end) = match self.index.get_bounds(&id) {
            Some(xs) => xs,
            None => return Ok(None)
        };

        let obj_type = self.packfile.read_bounds(start, end, output, backends)?;

        Ok(Some(obj_type))
    }
}
