use std;
use std::io::Write;

use crate::errors::Result;
use crate::id::Id;
use crate::objects::Type;
use crate::stores::{Queryable, StorageSet};

pub mod any;
pub mod index;
pub mod internal_type;
pub mod iter;
pub mod mmap;
mod read;

#[derive(Debug)]
pub struct IndexEntry {
    id: Id,
    offset: u64,
    crc32: u32,
    next: usize,
}

impl IndexEntry {
    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn crc32(&self) -> u32 {
        self.crc32
    }

    pub fn id(&self) -> &Id {
        &self.id
    }
}

pub struct Fanout([u32; 256]);

pub trait Packfile {
    fn read_bounds<W: Write, S: Queryable>(
        &self,
        start: u64,
        end: u64,
        output: &mut W,
        backends: &StorageSet<S>,
    ) -> Result<Type>;
}
