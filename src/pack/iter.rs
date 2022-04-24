use crypto::{ sha1::Sha1, digest::Digest };
use std::io::{ BufRead, Seek, Write };

use crate::pack::internal_type::PackfileType;
use crate::pack::read::PackfileEntryMeta;
use crate::errors::{ Result, ErrorKind };
use crate::pack::read::packfile_read;
use crate::id::Id;

pub struct PackfileIterator<R: BufRead + Seek + std::fmt::Debug> {
    index: u32,
    object_count: u32,
    stream: R,
    buffer: Vec<u8>,
    header_buffer: Vec<u8>,
    current_offset: u64,
}

impl<R: BufRead + Seek + std::fmt::Debug> PackfileIterator<R> {
    pub fn new(mut stream: R) -> Result<Self> {
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
            current_offset: 12,
            buffer: Vec::with_capacity(65535),
            header_buffer: Vec::with_capacity(128),
            stream
        })
    }
}

impl<R: BufRead + Seek + std::fmt::Debug> Iterator for PackfileIterator<R> {
    type Item = (u64, PackfileEntryMeta, Option<Id>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.object_count {
            return None
        }

        self.index += 1;
        self.buffer.clear();

        let offset = self.current_offset;
        let mut bytes_read = 0;
        let meta = packfile_read(
            &mut self.stream,
            &mut self.buffer,
            &mut bytes_read
        ).ok()?;

        self.current_offset += bytes_read;

        let id = if let PackfileType::Plain(object_type) = meta.expected_type() {
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

        Some((offset, meta, id))
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use super::PackfileIterator;

    #[test]
    fn does_it_blend() {
        let packfile = include_bytes!("../../fixtures/packfile");

        let packfile_iter: PackfileIterator<_> = PackfileIterator::new(Cursor::new(&packfile[..])).expect("failed to parse");
        for entry in packfile_iter {
            println!("entry: {:?}", entry);
        }
    }
}
