use flate2::bufread::DeflateDecoder;
use std::io::{BufReader, SeekFrom};
use std::path::{Path, PathBuf};
use std::io::prelude::*;
use packindex::Index;
use error::GitError;
use std::fs::File;
use id::Id;
use std;

use delta::{DeltaDecoder, OFS_DELTA, REF_DELTA};
use objects::Type;

pub trait ToReadable {
    type Reader;

    fn read(&self) -> Result<Reader>;
}

#[derive(Debug)]
pub struct Store<T: ToReadable> {
    reader: T,
    index: Index
}
// pack format is:
//
//      4 byte magic number ('P', 'A', 'C', 'K')
//      4 byte version number (2 or 3)
//      4 byte object count (N)
//      N objects
//      20 byte checksum
impl<T: Store> Store where T::Reader : std::io::Seek + std::io::Read {
    pub fn new(reader: T, idx: Option<Index>) -> Result<Store<T>> {
        let index = match idx {
            Some(i) => i,
            None => Store::build_index(&reader)?
        };

        Ok(Store {
            reader: reader,
            index: index
        })
    }

    pub fn build_index (reader: &reader) -> Result<Index> {
        Err(ErrorKind::NotImplemented.into())
    }

    pub fn read_bounds (&self, start: u64, end: u64) -> Result<(u8, Box<std::io::Read>)> {
        let handle = self.reader.read()?;
        let mut buffered_file = BufReader::new(handle);
        buffered_file.seek(SeekFrom::Start(start))?;
        let stream = buffered_file.take(end - start);

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

        match type_flag {
            _ => Err(GitError::Unknown),

            0...4 => {
                let mut zlib_header = [0u8; 2];
                object_stream.read_exact(&mut zlib_header)?;
                Ok((type_flag, Box::new(DeflateDecoder::new(object_stream))))
            },

            OFS_DELTA => {
                let mut take_one = object_stream.take(1);
                take_one.read_exact(&mut byte)?;
                let mut offset = (byte[0] & 0x7F) as u64;
                let mut original_stream = take_one.into_inner();

                while byte[0] & 0x80 > 0 {
                    offset += 1;
                    offset <<= 7;
                    take_one = original_stream.take(1);
                    take_one.read_exact(&mut byte)?;
                    offset += (byte[0] & 0x7F) as u64;
                    original_stream = take_one.into_inner();
                }

                let mut zlib_header = [0u8; 2];
                original_stream.read_exact(&mut zlib_header)?;
                let mut deflate_stream = DeflateDecoder::new(original_stream);
                let mut instructions = Vec::new();
                deflate_stream.read_to_end(&mut instructions);

                let (base_type, stream) = match self.read_bounds(start - offset, start) {
                    Ok(xs) => xs,
                    Err(e) => return Err(e)
                };

                Ok((base_type, Box::new(DeltaDecoder::new(&instructions, stream))))
            },

            REF_DELTA => {
                let mut ref_bytes = [0u8; 20];
                object_stream.read_exact(&mut ref_bytes)?;
                let id = Id::from_bytes(&ref_bytes);

                let mut zlib_header = [0u8; 2];
                object_stream.read_exact(&mut zlib_header)?;
                let mut deflate_stream = DeflateDecoder::new(object_stream);
                let mut instructions = Vec::new();
                deflate_stream.read_to_end(&mut instructions);

                let (t, base_stream) = match repository.get_object(&id)? {
                    Some(xs) => match xs {
                        Type::Commit(stream) => (1, stream),
                        Type::Tree(stream) => (2, stream),
                        Type::Blob(stream) => (3, stream),
                        Type::Tag(stream) => (4, stream)
                    },
                    None => return Err(GitError::Unknown)
                };

                Ok((t, Box::new(DeltaDecoder::new(&instructions, base_stream))))
            }
        }
    }

    fn get(&self, id: &Id) -> Result<Option<Type>, GitError> {
        let (start, end) = match self.index.get_bounds(&id) {
            Some(xs) => xs,
            None => return Ok(None)
        };
        let (t, stream) = self.read_bounds(start, end)?;
        match t {
            1 => Ok(Some(Type::Commit(stream))),
            2 => Ok(Some(Type::Tree(stream))),
            3 => Ok(Some(Type::Blob(stream))),
            4 => Ok(Some(Type::Tag(stream))),
            _ => Err(GitError::Unknown)
        }
    }
}
