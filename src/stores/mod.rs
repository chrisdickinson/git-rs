use std::collections::HashSet;

use crate::walk::commits::CommitIterator;
use crate::walk::tree::TreeIterator;
use crate::objects::{Type, Object};
use crate::errors::Result;
use crate::id::Id;

pub mod mmap_pack;
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

    pub fn tree(&self, id: &Id) -> TreeIterator {
        let result = match self.get_and_load(id) {
            Ok(xs) => xs,
            Err(_) => return TreeIterator::new(&self, vec![])
        };

        if result.is_none() {
            return TreeIterator::new(&self, vec![])
        }

        match result.unwrap() {
            Object::Commit(commit) => {
                match commit.tree() {
                    Some(tree) => self.tree(&tree),
                    None => TreeIterator::new(&self, vec![])
                }
            },
            Object::Tree(tree) => {
                TreeIterator::new(&self, vec![tree.into_iter()])
            },
            _ => {
                TreeIterator::new(&self, vec![])
            }
        }
    }

    pub fn get_and_load(&self, id: &Id) -> Result<Option<Object>> {
        match self.get(id)? {
            Some((typ, mut stream)) => Ok(Some(typ.load(&mut stream)?)),
            None => Ok(None)
        }
    }
}
