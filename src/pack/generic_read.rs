use std::io::{ BufReader, SeekFrom };
use flate2::bufread::ZlibDecoder;
use std::io::prelude::*;
use std::ops::Range;
use lru::LruCache;
use std;

use crate::delta::{ DeltaDecoder, DeltaDecoderStream, OFS_DELTA, REF_DELTA };
use crate::pack::internal_type::PackfileType;
use crate::stores::{ Storage, StorageSet };
use crate::errors::{ Result, ErrorKind };
use crate::objects::Type;
use crate::id::Id;

pub struct Unpacked {
    object_data: Vec<u8>,
    object_type: Type
}

impl Unpacked {
    pub fn new (object_type: Type, object_data: Vec<u8>) -> Unpacked {
        Unpacked {
            object_data,
            object_type
        }
    }
}

pub fn packfile_read<R: std::fmt::Debug + Read + BufRead + Seek, W: Write>(
    input: &mut R,
    output: &mut W
) -> Result<PackfileType> {
    let mut byte = [0u8; 1];
    input.read_exact(&mut byte)?;

    let obj_type = (byte[0] & 0x70) >> 4;
    let mut size = (byte[0] & 0xf) as u64;
    let mut count = 0;
    let mut continuation = byte[0] & 0x80;
    loop {
        if continuation < 1 {
            break
        }

        input.read_exact(&mut byte)?;
        continuation = byte[0] & 0x80;

        size |= ((byte[0] & 0x7f) as u64) << (4 + 7 * count);
        count += 1;
    }

    match obj_type {
        0...4 => {
            let mut deflated = ZlibDecoder::new(input);
            std::io::copy(&mut deflated, output)?;
            return Ok(PackfileType::Plain(obj_type));
        },

        OFS_DELTA => {
            input.read_exact(&mut byte)?;
            let mut offset = u64::from(byte[0] & 0x7F);

            while byte[0] & 0x80 > 0 {
                offset += 1;
                offset <<= 7;
                input.read_exact(&mut byte)?;
                offset += u64::from(byte[0] & 0x7F);
            }

            let mut deflate_stream = ZlibDecoder::new(input);
            let mut instructions = Vec::new();
            deflate_stream.read_to_end(&mut instructions)?;

            return Ok(PackfileType::OffsetDelta((offset, instructions)))
        },

        REF_DELTA => {
            let mut ref_bytes = [0u8; 20];
            input.read_exact(&mut ref_bytes)?;
            let id = Id::from(&ref_bytes);

            let mut deflate_stream = ZlibDecoder::new(input);
            let mut instructions = Vec::new();
            deflate_stream.read_to_end(&mut instructions)?;
            return Ok(PackfileType::RefDelta((id, instructions)))
        },

        _ => {
            Err(ErrorKind::BadLooseObject.into())
        }
    }
}

macro_rules! dbgr {
    ($val:expr) => {
        match $val {
            Ok(xs) => Ok(xs),
            Err(tmp) => {
                eprintln!("[{}:{}] {} = {:#?}",
                    file!(), line!(), stringify!($val), &tmp);
                Err(tmp)
            }
        }
    }
}

pub fn packfile_read_decompressed<R: std::fmt::Debug + Read + BufRead + Seek, W: Write>(
    input: &mut R,
    output: &mut W,
    backends: Option<&StorageSet>,
    lru_cache: Option<&mut LruCache<u64, Unpacked>>
) -> Result<(u64, Type)> {
    let first_position = input.seek(std::io::SeekFrom::Current(0))?;

    let packfile_type = packfile_read(input, output)?;
    let object_type = match packfile_type {
        PackfileType::Plain(t) => {
            PackfileType::Plain(t).into()
        },

        PackfileType::OffsetDelta((offset, instructions)) => {
            let mut intermediary = Vec::new();

            let (lru_cache, has_cached_item) = match lru_cache {
                Some(cache) => {
                    let entry = cache.get(&(first_position - offset));
                    match entry {
                        Some(unpacked) => {
                            (None, Some(unpacked))
                        },
                        None => {
                            (Some(cache), None)
                        }
                    }
                },
                None => {
                    (None, None)
                }
            };

            let object_type = if let Some(unpacked) = has_cached_item {

                intermediary.resize(unpacked.object_data.len(), 0u8);
                intermediary.copy_from_slice(&unpacked.object_data[..]);

                unpacked.object_type
            } else {
                let current_position = input.seek(std::io::SeekFrom::Current(0))?;
                input.seek(std::io::SeekFrom::Start(first_position - offset));

                let (_, output_type) = packfile_read_decompressed(
                    input,
                    &mut intermediary,
                    backends,
                    lru_cache
                )?;

                input.seek(std::io::SeekFrom::Start(current_position))?;
                output_type
            };


            let delta_decoder = DeltaDecoder::new(&instructions, intermediary)?;
            let mut stream: DeltaDecoderStream = delta_decoder.into();
            std::io::copy(&mut stream, output);
            object_type
        },

        PackfileType::RefDelta((id, instructions)) => {
            if backends.is_none() {
                return Err(ErrorKind::NeedStorageSet.into())
            }

            let (t, mut base_stream) = match backends.unwrap().get(&id)? {
                Some((xs, stream)) => (xs, stream),
                None => return Err(ErrorKind::CorruptedPackfile.into())
            };

            let mut base_buf = Vec::new();
            base_stream.read_to_end(&mut base_buf)?;
            let delta_decoder = DeltaDecoder::new(&instructions, base_buf)?;
            let mut stream: DeltaDecoderStream = delta_decoder.into();
            std::io::copy(&mut stream, output);
            t
        }
    };

    Ok((first_position, object_type))
}
