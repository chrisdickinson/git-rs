use byteorder::{BigEndian, ReadBytesExt};
use flate2::bufread::DeflateDecoder;
use std::io::{BufReader, SeekFrom};
use std::path::{Path, PathBuf};
use std::io::prelude::*;
use stores::Queryable;
use packindex::Index;
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

const OFS_DELTA: u8 = 6;
const REF_DELTA: u8 = 7;

pub struct DeltaDecoder {
}

impl DeltaDecoder {
    pub fn new (stream: Box<std::io::Read>, store: &Store, repo: &Repository) -> DeltaDecoder {
        DeltaDecoder {}
    }

    pub fn get_type(&self) -> u8 {
        0
    }
}

impl std::io::Read for DeltaDecoder {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        Ok(0)
    }
}

#[derive(Debug)]
pub struct Store {
    packfile_path: PathBuf,
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
    pub fn new(index_path: &Path, packfile_path: &Path) -> Result<Store, GitError> {
        let file = File::open(index_path)?;
        let buffered_file = BufReader::new(file);
        let index = Index::from(Box::new(buffered_file))?;

        Ok(Store {
            packfile_path: PathBuf::from(packfile_path),
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
        let file = File::open(&self.packfile_path)?;
        let mut buffered_file = BufReader::new(file);
        buffered_file.seek(SeekFrom::Start(start))?;
        let mut stream = buffered_file.take(end - start);

        // type + size bytes
        let mut continuation = 0;
        let mut type_flag = 0;
        let mut size_vec = Vec::new();
        let mut byte = [0u8; 1];

        let mut take_one = stream.take(1);
        take_one.read_exact(&mut byte)?;
        let mut original_stream = take_one.into_inner();
        continuation = byte[0] & 0x80;
        type_flag = (byte[0] & 0x70) >> 4;
        size_vec.push(byte[0] & 0x0f);
        loop {
            if continuation < 1 {
                break
            }

            take_one = original_stream.take(1);
            take_one.read_exact(&mut byte)?;
            original_stream = take_one.into_inner();
            continuation = byte[0] & 0x80;
            size_vec.push(byte[0] & 0x7f); 
        }
        let mut object_stream = original_stream;

        let count = size_vec.len();
        let mut size = match size_vec.pop() {
            Some(xs) => xs as u64,
            None => return Err(GitError::Unknown)
        };
        while size_vec.len() > 0 {
            let next = match size_vec.pop() {
                Some(xs) => xs as u64,
                None => return Err(GitError::Unknown)
            };
            size |= next << (4 + 7 * (count - size_vec.len()));
        }

        let (t, decoder_stream): (u8, Box<std::io::Read>) = if type_flag <= 4 {
            // grumble, grumble: we must strip the zlib header off of our content.
            let mut zlib_header = [0u8; 2];
            object_stream.read_exact(&mut zlib_header)?;
            (type_flag, Box::new(DeflateDecoder::new(object_stream)))
        } else {
            let decoder = DeltaDecoder::new(Box::new(object_stream), self, repo);
            let final_type = decoder.get_type();
            (final_type, Box::new(decoder))
        };

        if t == 1 {
            return Ok(Some(GitObject::CommitObject(Commit::from(id, decoder_stream))));
        } else if t == 2 {
            return Ok(Some(GitObject::TreeObject(Tree::from(id, decoder_stream))));
        } else if t == 3 {
            return Ok(Some(GitObject::BlobObject(Blob::from(id, decoder_stream))));
        }
        Err(GitError::Unknown)
    }
}
