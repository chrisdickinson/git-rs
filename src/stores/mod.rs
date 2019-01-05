use std::collections::HashSet;

use crate::walk::commits::CommitIterator;
use crate::objects::{Type, Object};
use crate::errors::Result;
use crate::id::Id;

pub mod loose;
pub mod pack;
pub mod fs;

pub trait Storage {
    fn get(&self, id: &Id, backends: &StorageSet) -> Result<Option<(Type, Box<std::io::Read>)>>;
}

pub struct StorageSet {
    backends: Vec<Box<Storage>>
}

impl StorageSet {
    pub fn new(backends: Vec<Box<Storage>>) -> StorageSet {
        StorageSet {
            backends
        }
    }

    pub fn get(&self, id: &Id) -> Result<Option<(Type, Box<std::io::Read>)>> {
        for store in &self.backends {
            let result = store.get(id, &self)?;
            if result.is_some() {
                return Ok(result);
            }
        }

        Ok(None)
    }

    pub fn commits(&self, id: &Id, seen: Option<HashSet<Id>>) -> CommitIterator {
        CommitIterator::new(&self, id, seen)
    }

    pub fn get_and_load(&self, id: &Id) -> Result<Option<Object>> {
        match self.get(id)? {
            Some((typ, mut stream)) => Ok(Some(typ.load(&mut stream)?)),
            None => Ok(None)
        }
    }
}
