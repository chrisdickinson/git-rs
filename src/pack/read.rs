use flate2::bufread::ZlibDecoder;
use std::convert::TryInto;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::fmt::Debug;

use crate::delta::{ DeltaDecoder, DeltaDecoderStream };
use crate::pack::internal_type::PackfileType;
use crate::stores::{ Queryable, StorageSet };
use crate::delta::{ OFS_DELTA, REF_DELTA };
use crate::errors::{ Result, ErrorKind };
use crate::objects::Type;

#[derive(Debug)]
pub struct PackfileEntryMeta {
    expected_type: PackfileType,
    expected_size: u64
}

impl PackfileEntryMeta {
    pub fn expected_type(&self) -> &PackfileType {
        &self.expected_type
    }

    pub fn expected_size(&self) -> u64 {
        self.expected_size
    }

    pub fn decompress<R, W, S>(self, initial: u64, input: &mut R, output: &mut W, backends: Option<&StorageSet<S>>) -> Result<Type>
        where R: Debug + Read + BufRead + Seek,
              W: Write,
              S: Queryable {
        Ok(match self.expected_type {
            PackfileType::Plain(t) => {
                t
            },

            PackfileType::OffsetDelta((offset, instructions)) => {
                let mut intermediary = Vec::new();
                let current_position = input.seek(SeekFrom::Current(0))?;
                let object_start = initial - offset;
                input.seek(SeekFrom::Start(object_start))?;

                let meta = packfile_read(
                    input,
                    &mut intermediary,
                    &mut 0,
                )?;

                let object_type = meta.decompress(
                    object_start,
                    input,
                    &mut intermediary,
                    backends
                )?;

                input.seek(SeekFrom::Start(current_position))?;
                let delta_decoder = DeltaDecoder::new(&instructions, intermediary)?;
                let mut stream: DeltaDecoderStream = delta_decoder.into();
                std::io::copy(&mut stream, output)?;
                object_type
            },

            PackfileType::RefDelta((id, instructions)) => {
                if backends.is_none() {
                    return Err(ErrorKind::NeedStorageSet.into())
                }

                let mut base_data = Vec::new();
                let t = match backends.unwrap().get(&id, &mut base_data)? {
                    Some(xs) => xs,
                    None => return Err(ErrorKind::CorruptedPackfile.into())
                };

                let delta_decoder = DeltaDecoder::new(&instructions, base_data)?;
                let mut stream: DeltaDecoderStream = delta_decoder.into();
                std::io::copy(&mut stream, output)?;
                t
            }
        })

    }
}

pub fn packfile_read<R: BufRead, W: Write>(
    input: &mut R,
    output: &mut W,
    read_bytes: &mut u64
) -> Result<PackfileEntryMeta> {
    let mut byte = [0u8; 1];
    input.read_exact(&mut byte)?;

    let obj_type = (byte[0] & 0x70) >> 4;
    let mut size = (byte[0] & 0xf) as u64;
    let mut count = 0;
    let mut continuation = byte[0] & 0x80;

    while continuation > 0 {
        input.read_exact(&mut byte)?;
        continuation = byte[0] & 0x80;

        size |= ((byte[0] & 0x7f) as u64) << (4 + 7 * count);
        count += 1;
    }

    match obj_type {
        0..=4 => {
            let mut deflate_stream = ZlibDecoder::new(input);
            std::io::copy(&mut deflate_stream, output)?;
            *read_bytes = 1 + count + deflate_stream.total_in();
            Ok(PackfileEntryMeta {
                expected_type: PackfileType::Plain(obj_type.try_into()?),
                expected_size: size
            })
        },

        OFS_DELTA => {
            input.read_exact(&mut byte)?;
            let mut offset = u64::from(byte[0] & 0x7F);

            while byte[0] & 0x80 > 0 {
                offset += 1;
                offset <<= 7;
                input.read_exact(&mut byte)?;
                offset += u64::from(byte[0] & 0x7F);
                count += 1;
            }

            let mut deflate_stream = ZlibDecoder::new(input);
            let mut instructions = Vec::new();
            deflate_stream.read_to_end(&mut instructions)?;

            *read_bytes = 2 + count + deflate_stream.total_in();
            Ok(PackfileEntryMeta {
                expected_type: PackfileType::OffsetDelta((offset, instructions)),
                expected_size: size
            })
        },

        REF_DELTA => {
            let mut ref_bytes = [0u8; 20];
            input.read_exact(&mut ref_bytes)?;
            let id = ref_bytes.into();

            let mut deflate_stream = ZlibDecoder::new(input);
            let mut instructions = Vec::new();
            deflate_stream.read_to_end(&mut instructions)?;
            *read_bytes = 21 + count + deflate_stream.total_in();
            Ok(PackfileEntryMeta {
                expected_type: PackfileType::RefDelta((id, instructions)),
                expected_size: size
            })
        },

        _ => {
            Err(ErrorKind::BadLooseObject.into())
        }
    }
}
