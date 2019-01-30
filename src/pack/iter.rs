use crypto::{ sha1::Sha1, digest::Digest };
use std::io::{ BufRead, Seek, Read };

use crate::pack::generic_read::packfile_read;
use crate::errors::{ Result, ErrorKind };
use crate::objects::Type;
use crate::pack::IndexEntry;
use crate::delta::{ DeltaDecoder, DeltaDecoderStream };
use crate::pack::internal_type::PackfileType;
use crate::stores::StorageSet;
use crate::id::Id;

pub struct PackfileIterator<'a, R: BufRead + Seek + std::fmt::Debug> {
    index: u32,
    version: u32,
    object_count: u32,
    stream: R,
    last_object: [Vec<u8>; 2],
    last_object_type: [Type; 2],
    storage_set: Option<&'a StorageSet>
}

impl<'a, R: BufRead + Seek + std::fmt::Debug> PackfileIterator<'a, R> {
    pub fn new(mut stream: R, storage_set: Option<&'a StorageSet>) -> Result<Self> {
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
            last_object: [vec![], vec![]],
            last_object_type: [Type::Blob, Type::Blob],
            storage_set,
            stream
        })
    }
}

impl<'a, R: BufRead + Seek + std::fmt::Debug> Iterator for PackfileIterator<'a, R> {
    type Item = IndexEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.object_count {
            return None
        }

        self.index += 1;
        let offset = self.stream.seek(std::io::SeekFrom::Current(0)).ok()?;
        let current_object_idx = if self.index & 1 == 0 { 0 } else { 1 };
        self.last_object[current_object_idx].clear();
        let packfile_type = packfile_read(
            &mut self.stream,
            &mut self.last_object[current_object_idx]
        ).ok()?;

        let id = match packfile_type {
            PackfileType::Plain(t) => {
                let mut hash = Sha1::new();

                let object_type: Type = PackfileType::Plain(t).into();

                hash.input(format!("{} {}", object_type.as_str(), self.last_object[current_object_idx].len()).as_bytes());
                hash.input(&[0u8]);
                hash.input(&(self.last_object[current_object_idx])[..]);
                self.last_object_type[current_object_idx] = object_type;
                let mut id_output = [0u8; 20];
                hash.result(&mut id_output);

                Id::from(&id_output[..])
            },

            PackfileType::OffsetDelta((offset, instructions)) => {
                let last_object_idx = if self.index & 1 == 0 { 1 } else { 0 };
                self.last_object_type[current_object_idx] = self.last_object_type[last_object_idx];

                let delta_decoder = DeltaDecoder::new(&instructions, self.last_object[last_object_idx].split_off(0)).ok()?;
                let mut dds_stream: DeltaDecoderStream = delta_decoder.into();
                dds_stream.read_to_end(&mut self.last_object[current_object_idx]).ok()?;
                let mut hash = Sha1::new();

                hash.input(self.last_object_type[current_object_idx].as_str().as_bytes());
                hash.input(b" ");
                hash.input(format!("{}", self.last_object[current_object_idx].len()).as_bytes());
                hash.input(&[0u8]);
                hash.input(&(self.last_object[current_object_idx])[..]);
                let mut id_output = [0u8; 20];
                hash.result(&mut id_output);

                Id::from(&id_output[..])
            },

            PackfileType::RefDelta((id, instructions)) => {
                let last_object_idx = if self.index & 1 == 0 { 1 } else { 0 };
                self.last_object_type[current_object_idx] = self.last_object_type[last_object_idx];
                if self.storage_set.is_none() {
                    return None
                }

                let (_, mut base_stream) = self.storage_set.unwrap().get(&id).ok()??;

                let mut base_buf = Vec::new();
                base_stream.read_to_end(&mut self.last_object[last_object_idx]).ok()?;
                let delta_decoder = DeltaDecoder::new(&instructions, base_buf).ok()?;
                let mut dds_stream: DeltaDecoderStream = delta_decoder.into();
                dds_stream.read_to_end(&mut self.last_object[current_object_idx]).ok()?;

                let mut hash = Sha1::new();

                hash.input(self.last_object_type[current_object_idx].as_str().as_bytes());
                hash.input(b" ");
                hash.input(format!("{}", self.last_object[current_object_idx].len()).as_bytes());
                hash.input(&[0u8]);
                hash.input(&(self.last_object[current_object_idx])[..]);
                let mut id_output = [0u8; 20];
                hash.result(&mut id_output);

                Id::from(&id_output[..])
            }
        };

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
        for entry in PackfileIterator::new(Cursor::new(&packfile[..]), None).expect("failed to parse") {
            println!("entry: {:?}", entry);
        }
    }
}
