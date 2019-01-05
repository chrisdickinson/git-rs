use std::io::{ BufReader, SeekFrom };
use flate2::bufread::DeflateDecoder;
use std::path::{ Path, PathBuf };
use std::io::prelude::*;
use std::fs::File;
use std;

use crate::delta::{ DeltaDecoder, DeltaDecoderStream, OFS_DELTA, REF_DELTA };
use crate::stores::{ Storage, StorageSet };
use crate::errors::{ Result, ErrorKind };
use crate::packindex::Index;
use crate::objects::Type;
use crate::id::Id;

pub type GetObject = Fn(&Id) -> Result<Option<(Type, Box<std::io::Read>)>>;

pub struct Store<R> {
    read: Box<Fn() -> Result<R>>,
    index: Index
}

// pack format is:
//
//      4 byte magic number ('P', 'A', 'C', 'K')
//      4 byte version number (2 or 3)
//      4 byte object count (N)
//      N objects
//      20 byte checksum

fn build_index(_reader: Box<std::io::Read>) -> Result<Index> {
    Err(ErrorKind::NotImplemented.into())
}

impl<R: std::io::Read + std::io::Seek + 'static> Store<R> {
    pub fn new<C>(func: C, index: Option<Index>) -> Result<Self>
        where C: Fn() -> Result<R> + 'static {

        let idx = match index {
            Some(xs) => xs,
            None => build_index(Box::new(func()?))?
        };

        Ok(Store {
            read: Box::new(func),
            index: idx
        })
    }

    pub fn read_bounds (&self, start: u64, end: u64, backends: &StorageSet) -> Result<(u8, Box<std::io::Read>)> {
        let handle = (self.read)()?;
        let mut buffered_file = BufReader::new(handle);
        buffered_file.seek(SeekFrom::Start(start))?;

        let stream = buffered_file.take(end - start);

        // type + size bytes
        let mut continuation;
        let mut size_vec = Vec::new();
        let mut byte = [0u8; 1];

        let mut take_one = stream.take(1);
        take_one.read_exact(&mut byte)?;
        let mut original_stream = take_one.into_inner();
        continuation = byte[0] & 0x80;
        let type_flag = (byte[0] & 0x70) >> 4;
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
        let mut _size = match size_vec.pop() {
            Some(xs) => xs as u64,
            None => return Err(ErrorKind::CorruptedPackfile.into())
        };
        while size_vec.len() > 0 {
            let next = match size_vec.pop() {
                Some(xs) => xs as u64,
                None => return Err(ErrorKind::CorruptedPackfile.into())
            };
            _size |= next << (4 + 7 * (count - size_vec.len()));
        }

        match type_flag {
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
                deflate_stream.read_to_end(&mut instructions)?;

                let (base_type, mut stream) = self.read_bounds(start - offset, start, backends)?;
                let mut base_buf = Vec::new();

                stream.read_to_end(&mut base_buf)?;
                let delta_decoder = DeltaDecoder::new(&instructions, base_buf)?;
                let dd_stream: DeltaDecoderStream = delta_decoder.into();
                Ok((base_type, Box::new(dd_stream)))
            },

            REF_DELTA => {
                let mut ref_bytes = [0u8; 20];
                object_stream.read_exact(&mut ref_bytes)?;
                let id = Id::from(&ref_bytes);

                let mut zlib_header = [0u8; 2];
                object_stream.read_exact(&mut zlib_header)?;
                let mut deflate_stream = DeflateDecoder::new(object_stream);
                let mut instructions = Vec::new();
                deflate_stream.read_to_end(&mut instructions)?;

                let (t, mut base_stream) = match backends.get(&id)? {
                    Some((xs, stream)) => match xs {
                        Type::Commit => (1, stream),
                        Type::Tree => (2, stream),
                        Type::Blob => (3, stream),
                        Type::Tag => (4, stream)
                    },
                    None => return Err(ErrorKind::CorruptedPackfile.into())
                };

                let mut base_buf = Vec::new();

                base_stream.read_to_end(&mut base_buf)?;


                let delta_decoder = DeltaDecoder::new(&instructions, base_buf)?;
                let stream: DeltaDecoderStream = delta_decoder.into();
                Ok((t, Box::new(stream)))
            },

            _ => {
                return Err(ErrorKind::BadLooseObject.into())
            }
        }
    }
}


impl<R: std::io::Read + std::io::Seek + 'static> Storage for Store<R> {
    fn get(&self, id: &Id, backends: &StorageSet) -> Result<Option<(Type, Box<std::io::Read>)>> {
        let (start, end) = match self.index.get_bounds(&id) {
            Some(xs) => xs,
            None => return Ok(None)
        };

        let (t, stream) = self.read_bounds(start, end, backends)?;
        let typed = match t {
            1 => Type::Commit,
            2 => Type::Tree,
            3 => Type::Blob,
            4 => Type::Tag,
            _ => return Err(ErrorKind::CorruptedPackfile.into())
        };

        Ok(Some((typed, stream)))
    }
}

#[cfg(test)]
mod tests {

    use super::Index;
    use super::Store;
    use super::Id;
    use std::io::Cursor;
    use crate::objects::Object;
    use crate::stores::{ Storage, StorageSet };

    #[test]
    fn can_load() {
        let bytes = include_bytes!("../../fixtures/pack_index");

        let idx = Index::from(&mut bytes.as_ref()).expect("bad index");
        let pack = Store::new(|| Ok(Cursor::new(include_bytes!("../../fixtures/packfile") as &[u8])), Some(idx)).expect("bad packfile");
        let storage_set = StorageSet::new(Vec::new());

        let id = Id::from_str("872e26b3fbebe64a2a85b271fed6916b964b4fde").unwrap();
        let (kind, mut stream) = pack.get(&id, &storage_set).expect("failure").unwrap();

        let obj = kind.load(&mut stream).expect("failed to load object");

        match obj {
            Object::Commit(commit) => {
                let msg = std::str::from_utf8(commit.message()).expect("invalid string");
                assert_eq!(msg, "ok\n");
            },
            _ => panic!("expected commit")
        };
    }
}
