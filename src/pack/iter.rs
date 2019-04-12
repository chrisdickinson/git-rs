use crypto::{ sha1::Sha1, digest::Digest };
use std::io::{ BufRead, Seek, Write };

use crate::stores::{ Queryable, StorageSet };
use crate::errors::{ Result, ErrorKind };
use crate::pack::read::packfile_read;
use crate::id::Id;

pub struct PackfileIterator<'a, R: BufRead + Seek + std::fmt::Debug, S: Queryable> {
    index: u32,
    object_count: u32,
    stream: R,
    buffer: Vec<u8>,
    header_buffer: Vec<u8>,
    current_offset: u64,
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

        let version = u32::from_be_bytes(version_bytes);
        match version {
            2 | 3 => (),
            _ => return Err(ErrorKind::NotImplemented.into())
        };

        let mut object_count_bytes = [0u8; 4];
        stream.read_exact(&mut object_count_bytes)?;
        let object_count = u32::from_be_bytes(object_count_bytes);

        Ok(PackfileIterator {
            index: 0,
            object_count,
            storage_set,
            current_offset: 12,
            buffer: Vec::with_capacity(65535),
            header_buffer: Vec::with_capacity(128),
            stream
        })
    }
}

use crate::pack::internal_type::PackfileType;
use crate::objects::Type;

impl<'a, R: BufRead + Seek + std::fmt::Debug, S: Queryable> Iterator for PackfileIterator<'a, R, S> {
    type Item = (u64, PackfileType, Option<Id>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.object_count {
            return None
        }

        self.index += 1;
        self.buffer.clear();

        let offset = self.current_offset;
        let mut bytes_read = 0;
        let packfile_type = packfile_read(
            &mut self.stream,
            &mut self.buffer,
            &mut bytes_read
        ).ok()?;

        self.current_offset += bytes_read;

        let id = if let PackfileType::Plain(ident) = packfile_type {
            let object_type: Type = PackfileType::Plain(ident).into();
            let mut hash = Sha1::new();
            self.header_buffer.clear();
            write!(&mut self.header_buffer, "{} {}\0", object_type.as_str(), self.buffer.len()).ok()?;
            hash.input(&(self.header_buffer)[..]);
            hash.input(&(self.buffer)[..]);

            let mut id_output = [0u8; 20];
            hash.result(&mut id_output);
            let id: Id = id_output.into();
            Some(id)
        } else {
            None
        };

        Some((offset, packfile_type, id))
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
