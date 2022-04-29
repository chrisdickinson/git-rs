use std::collections::HashSet;
use std::io::Cursor;
use std::io::Write;

use crate::walk::commits::CommitIterator;
use crate::walk::tree::TreeIterator;
use crate::objects::{Type, Object};
use crate::errors::Result;
use crate::id::Id;

pub mod loose;
pub mod pack;
pub mod fs;

pub trait Queryable {
    fn get<W: Write, S: Queryable>(&self, id: &Id, output: &mut W, backends: &StorageSet<S>) -> Result<Option<Type>>;
}

impl Queryable for () {
    fn get<W: Write, S: Queryable>(&self, _id: &Id, _output: &mut W, _backends: &StorageSet<S>) -> Result<Option<Type>> {
        Ok(None)
    }
}

impl<Q: Queryable> Queryable for (Q,) {
    fn get<W: Write, S: Queryable>(&self, id: &Id, output: &mut W, backends: &StorageSet<S>) -> Result<Option<Type>> {
        self.0.get(id, output, backends)
    }
}

impl<H: Queryable, T: Queryable> Queryable for (H, T) {
    fn get<W: Write, S: Queryable>(&self, id: &Id, output: &mut W, backends: &StorageSet<S>) -> Result<Option<Type>> {
        let opt = self.0.get(id, output, backends)?;
        if opt.is_some() {
            return Ok(opt)
        }

        self.1.get(id, output, backends)
    }
}

impl<Q: Queryable> Queryable for Vec<Q> {
    fn get<W: Write, S: Queryable>(&self, id: &Id, output: &mut W, backends: &StorageSet<S>) -> Result<Option<Type>> {
        for queryable in self {
            let opt = queryable.get(id, output, backends)?;
            if opt.is_some() {
                return Ok(opt)
            }
        }

        Ok(None)
    }
}

pub struct StorageSet<Q: Queryable> {
    backend: Q
}

impl<Q: Queryable> StorageSet<Q> {
    pub fn new(backend: Q) -> StorageSet<Q> {
        StorageSet {
            backend
        }
    }

    pub fn get<W: Write>(&self, id: &Id, output: &mut W) -> Result<Option<Type>> {
        self.backend.get(id, output, self)
    }

    pub fn commits(&self, id: &Id, seen: Option<HashSet<Id>>) -> CommitIterator<Q> {
        CommitIterator::new(self, id, seen)
    }

    pub fn tree(&self, id: &Id) -> TreeIterator<Q> {
        let result = match self.get_and_load(id) {
            Ok(xs) => xs,
            Err(_) => return TreeIterator::new(self, vec![])
        };

        if result.is_none() {
            return TreeIterator::new(self, vec![])
        }

        match result.unwrap() {
            Object::Commit(commit) => {
                match commit.tree() {
                    Some(tree) => self.tree(&tree),
                    None => TreeIterator::new(self, vec![])
                }
            },
            Object::Tree(tree) => {
                TreeIterator::new(self, vec![tree.into_iter()])
            },
            _ => {
                TreeIterator::new(self, vec![])
            }
        }
    }

    pub fn get_and_load(&self, id: &Id) -> Result<Option<Object>> {
        let mut data = Vec::new();
        match self.get(id, &mut data)? {
            Some(typ) => Ok(Some(typ.load(&mut Cursor::new(&data))?)),
            None => Ok(None)
        }
    }
}
