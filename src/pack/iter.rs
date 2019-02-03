use crypto::{ sha1::Sha1, digest::Digest };
use std::io::{ BufRead, Seek, Read };
use lru::LruCache;

use crate::pack::read::{ packfile_read, packfile_read_decompressed, Unpacked };
use crate::errors::{ Result, ErrorKind };
use crate::objects::Type;
use crate::pack::IndexEntry;
use crate::delta::{ DeltaDecoder, DeltaDecoderStream };
use crate::pack::internal_type::PackfileType;
use crate::stores::{ Queryable, StorageSet };
use crate::id::Id;

pub struct PackfileIterator<'a, R: BufRead + Seek + std::fmt::Debug, S: Queryable> {
    index: u32,
    version: u32,
    object_count: u32,
    stream: R,
    buffer: Vec<u8>,
    cache: LruCache<u64, Unpacked>,
    storage_set: Option<&'a StorageSet<S>>
}

impl<'a, R: BufRead + Seek + std::fmt::Debug, S: Queryable> PackfileIterator<'a, R, S> {
    pub fn new(mut stream: R, storage_set: Option<&'a StorageSet<S>>) -> Result<Self> {
        let mut magic = [0u8; 4];
        stream.read_exact(&mut magic)?;

        if &magic != b"PACK" {
            return Err(ErrorKind::CorruptedPackfile.into())
        }

        let mut version_bytes = [0u8; 4];
        stream.read_exact(&mut version_bytes)?;

        let version = unsafe { std::mem::transmute::<[u8; 4], u32>(version_bytes) }.to_be();
        match version {
            2 | 3 => (),
            _ => return Err(ErrorKind::NotImplemented.into())
        };

        let mut object_count_bytes = [0u8; 4];
        stream.read_exact(&mut object_count_bytes)?;
        let object_count = unsafe { std::mem::transmute::<[u8; 4], u32>(object_count_bytes) }.to_be();

        Ok(PackfileIterator {
            index: 0,
            version,
            object_count,
            storage_set,
            buffer: Vec::new(),
            cache: LruCache::new(4096),
            stream
        })
    }
}

impl<'a, R: BufRead + Seek + std::fmt::Debug, S: Queryable> Iterator for PackfileIterator<'a, R, S> {
    type Item = IndexEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.object_count {
            return None
        }

        self.index += 1;
        self.buffer.clear();
        let (offset, object_type) = packfile_read_decompressed(
            &mut self.stream,
            &mut self.buffer,
            self.storage_set,
            Some(&mut self.cache)
        ).ok()?;

        let mut hash = Sha1::new();
        hash.input(format!("{} {}\0", object_type.as_str(), self.buffer.len()).as_bytes());
        hash.input(&(self.buffer)[..]);
        let mut id_output = [0u8; 20];
        hash.result(&mut id_output);
        let id = Id::from(&id_output[..]);

        self.cache.put(offset, Unpacked::new(
            object_type,
            self.buffer.clone()
        ));

        Some(IndexEntry {
            id,
            offset,
            crc32: 0xdeadbeef, // dunno what we're crc'ing, yet.
            next: 0
        })
    }
}

#[cfg(test)]
mod tests {
    use std::io::{ Read, Cursor };
    use super::PackfileIterator;

    #[test]
    fn does_it_blend() {
        let packfile = include_bytes!("../../fixtures/packfile");

        let packfile_iter: PackfileIterator<_, ()> = PackfileIterator::new(Cursor::new(&packfile[..]), None).expect("failed to parse");
        for entry in packfile_iter {
            println!("entry: {:?}", entry);
        }
    }
}
