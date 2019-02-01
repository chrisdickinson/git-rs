use std::io::{ Cursor, Write };

use crate::stores::{ Storage, StorageSet };
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

impl<P: Packfile> Storage for Store<P> {
    fn get(&self, id: &Id, backends: &StorageSet) -> Result<Option<(Type, Box<std::io::Read>)>> {
        let (start, end) = match self.index.get_bounds(&id) {
            Some(xs) => xs,
            None => return Ok(None)
        };

        let mut stream = Vec::new();
        let obj_type = self.packfile.read_bounds(start, end, &mut stream, backends)?;

        Ok(Some((obj_type, Box::new(Cursor::new(stream)))))
    }
}
