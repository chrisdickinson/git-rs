use std::io::Write;
use std;

use crate::stores::{ Queryable, StorageSet };
use crate::errors::Result;
use crate::objects::Type;
use crate::id::Id;

pub mod index;
pub mod mmap;
pub mod any;
pub mod iter;
pub mod internal_type;
mod read;

#[derive(Debug)]
pub struct IndexEntry {
    id: Id,
    offset: u64,
    crc32: u32,
    next: usize
}

impl IndexEntry {
    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn crc32(&self) -> u32 {
        self.crc32
    }

    pub fn next(&self) -> usize {
        self.next
    }

    pub fn id(&self) -> &Id {
        &self.id
    }
}

pub struct Fanout ([u32; 256]);

pub trait Packfile {
    fn read_bounds<W: Write, S: Queryable>(&self, start: u64, end: u64, output: &mut W, backends: &StorageSet<S>) -> Result<Type>;
}
