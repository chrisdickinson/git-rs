use crate::errors::{ Result, ErrorKind };
use crate::pack::IndexEntry;

pub struct PackfileIterator {
}

impl PackfileIterator {
    pub fn new() -> PackfileIterator {
        PackfileIterator { }
    }
}

impl Iterator for PackfileIterator {
    type Item = Result<IndexEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

