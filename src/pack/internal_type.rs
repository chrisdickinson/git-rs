use std;
use std::fmt::Debug;
use std::io::prelude::*;
use std::io::SeekFrom;

use crate::delta::{DeltaDecoder, DeltaDecoderStream};
use crate::errors::{ErrorKind, Result};
use crate::id::Id;
use crate::objects::Type;
use crate::pack::read::packfile_read;
use crate::stores::{Queryable, StorageSet};

#[derive(Debug)]
pub enum PackfileType {
    Plain(u8),
    OffsetDelta((u64, Vec<u8>)),
    RefDelta((Id, Vec<u8>)),
}

impl PackfileType {
    pub fn decompress<R, W, S>(
        self,
        initial: u64,
        input: &mut R,
        output: &mut W,
        backends: Option<&StorageSet<S>>,
    ) -> Result<Type>
    where
        R: Debug + Read + BufRead + Seek,
        W: Write,
        S: Queryable,
    {
        Ok(match self {
            PackfileType::Plain(t) => PackfileType::Plain(t).into(),

            PackfileType::OffsetDelta((offset, instructions)) => {
                let mut intermediary = Vec::new();
                let current_position = input.seek(SeekFrom::Current(0))?;
                let object_start = initial - offset;
                input.seek(SeekFrom::Start(object_start))?;

                let object_type = packfile_read(input, &mut intermediary, &mut 0)?.decompress(
                    object_start,
                    input,
                    &mut intermediary,
                    backends,
                )?;

                input.seek(SeekFrom::Start(current_position))?;
                let delta_decoder = DeltaDecoder::new(&instructions, intermediary)?;
                let mut stream: DeltaDecoderStream = delta_decoder.into();
                std::io::copy(&mut stream, output)?;
                object_type
            }

            PackfileType::RefDelta((id, instructions)) => {
                if backends.is_none() {
                    return Err(ErrorKind::NeedStorageSet.into());
                }

                let mut base_data = Vec::new();
                let t = match backends.unwrap().get(&id, &mut base_data)? {
                    Some(xs) => xs,
                    None => return Err(ErrorKind::CorruptedPackfile.into()),
                };

                let delta_decoder = DeltaDecoder::new(&instructions, base_data)?;
                let mut stream: DeltaDecoderStream = delta_decoder.into();
                std::io::copy(&mut stream, output)?;
                t
            }
        })
    }
}
