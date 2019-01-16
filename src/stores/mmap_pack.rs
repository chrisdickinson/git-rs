use flate2::bufread::DeflateDecoder;
use std::io::prelude::*;
use memmap::Mmap;
use std;

use crate::delta::{ DeltaDecoder, DeltaDecoderStream, OFS_DELTA, REF_DELTA };
use crate::stores::{ Storage, StorageSet };
use crate::errors::{ Result, ErrorKind };
use crate::packindex::Index;
use crate::objects::Type;
use crate::id::Id;

pub type GetObject = Fn(&Id) -> Result<Option<(Type, Box<std::io::Read>)>>;

pub struct Store {
    mmap: Mmap,
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
    pub fn new(mmap: Mmap, index: Option<Index>) -> Result<Self> {
        let idx = match index {
            Some(xs) => xs,
            None => return Err(ErrorKind::CorruptedPackfile.into())
        };

        Ok(Store {
            mmap,
            index: idx
        })
    }

    pub fn read_bounds (&self, start: u64, end: u64, backends: &StorageSet) -> Result<(u8, Box<std::io::Read>)> {
        let mut idx = start as usize;

        // type + size bytes
        let mut continuation;
        let mut size_vec = Vec::new();

        let byt = self.mmap[idx];
        continuation = byt & 0x80;
        let type_flag = (byt & 0x70) >> 4;
        size_vec.push(byt & 0x0f);
        idx += 1;
        loop {
            if continuation < 1 {
                break
            }

            let byt = self.mmap[idx];
            continuation = byt & 0x80;
            size_vec.push(byt & 0x7f);
            idx += 1;
        }

        let count = size_vec.len();
        let mut _size = match size_vec.pop() {
            Some(xs) => u64::from(xs),
            None => return Err(ErrorKind::CorruptedPackfile.into())
        };

        while !size_vec.is_empty() {
            let next = match size_vec.pop() {
                Some(xs) => u64::from(xs),
                None => return Err(ErrorKind::CorruptedPackfile.into())
            };
            _size |= next << (4 + 7 * (count - size_vec.len()));
        }

        match type_flag {
            0...4 => {
                let mut stream = DeflateDecoder::new(&self.mmap[idx + 2..= end as usize]);
                let mut results = Vec::new();
                stream.read_to_end(&mut results)?;
                Ok((type_flag, Box::new(std::io::Cursor::new(results))))
            },

            OFS_DELTA => {
                let mut byt = self.mmap[idx];
                let mut offset = u64::from(byt & 0x7F);
                idx += 1;

                while byt & 0x80 > 0 {
                    offset += 1;
                    offset <<= 7;
                    byt = self.mmap[idx];
                    idx += 1;
                    offset += u64::from(byt & 0x7F);
                }

                let mut deflate_stream = DeflateDecoder::new(&self.mmap[idx + 2 ..= end as usize]);
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
                let id = Id::from(&self.mmap[idx .. idx + 20]);

                // 22: 20 bytes of id, 2 bytes of zlib header.
                let mut deflate_stream = DeflateDecoder::new(&self.mmap[idx + 22 ..= end as usize]);
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
                Err(ErrorKind::BadLooseObject.into())
            }
        }
    }
}


impl Storage for Store {
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

        let id: Id = "872e26b3fbebe64a2a85b271fed6916b964b4fde".parse().unwrap();
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
