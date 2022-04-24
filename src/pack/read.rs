use flate2::bufread::ZlibDecoder;
use std::convert::TryInto;
use std::io::prelude::*;

use crate::pack::internal_type::PackfileType;
use crate::delta::{ OFS_DELTA, REF_DELTA };
use crate::errors::{ Result, ErrorKind };
use crate::id::Id;

pub fn packfile_read<R: BufRead, W: Write>(
    input: &mut R,
    output: &mut W,
    read_bytes: &mut u64
) -> Result<PackfileType> {
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

    drop(size);

    match obj_type {
        0..=4 => {
            let mut deflate_stream = ZlibDecoder::new(input);
            std::io::copy(&mut deflate_stream, output)?;
            *read_bytes = 1 + count + deflate_stream.total_in();
            return Ok(PackfileType::Plain(obj_type.try_into()?));
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
            return Ok(PackfileType::OffsetDelta((offset, instructions)))
        },

        REF_DELTA => {
            let mut ref_bytes = [0u8; 20];
            input.read_exact(&mut ref_bytes)?;
            let id = Id::from(&ref_bytes);

            let mut deflate_stream = ZlibDecoder::new(input);
            let mut instructions = Vec::new();
            deflate_stream.read_to_end(&mut instructions)?;
            *read_bytes = 21 + count + deflate_stream.total_in();
            return Ok(PackfileType::RefDelta((id, instructions)))
        },

        _ => {
            Err(ErrorKind::BadLooseObject.into())
        }
    }
}
