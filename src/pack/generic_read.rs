use std::io::{ BufReader, SeekFrom };
use flate2::bufread::ZlibDecoder;
use std::io::prelude::*;
use std::ops::Range;
use std;

use crate::delta::{ OFS_DELTA, REF_DELTA };
use crate::stores::{ Storage, StorageSet };
use crate::errors::{ Result, ErrorKind };
use crate::pack::internal_type::PackfileType;
use crate::id::Id;

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
