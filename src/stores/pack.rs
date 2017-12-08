use byteorder::{BigEndian, ReadBytesExt};
use std::path::{Path, PathBuf};
use stores::index::Index;
use std::io::prelude::*;
use std::io::BufReader;
use stores::Queryable;
use error::GitError;
use std::fs::File;
use std::io;
use id::Id;
use std;

use repository::Repository;
use objects::commit::Commit;
use objects::tree::Tree;
use objects::blob::Blob;
use objects::GitObject;

#[derive(Debug)]
pub struct Store {
    name: PathBuf,
    index: Index
}
// pack format is:
//
//      4 byte magic number ('P', 'A', 'C', 'K')
//      4 byte version number (2 or 3)
//      4 byte object count (N)
//      N objects
//      20 byte checksum
impl Store {
    pub fn from(path: &Path) -> Result<Store, GitError> {
        let file = File::open(path)?;
        let buffered_file = BufReader::new(file);
        let index = Index::from(Box::new(buffered_file))?;

        Ok(Store {
            name: PathBuf::from(path),
            index: index
        })
    }
}

impl Queryable for Store {
    fn get(&self, repo: &Repository, id: &Id) -> Result<Option<GitObject>, GitError> {
        let (start, end) = match self.index.get_bounds(&id) {
            Some(xs) => xs,
            None => return Ok(None)
        };




        Ok(None)
    }
}
