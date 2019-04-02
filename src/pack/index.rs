use byteorder::{BigEndian, ReadBytesExt};
use crc::crc32::{self, Digest as CRCDigest, Hasher32};
use crypto::{digest::Digest, sha1::Sha1};
use rayon::prelude::*;
use std::fmt::Debug;
use std::io::prelude::*;
use std::io::{Cursor, SeekFrom};

use crate::errors::{ErrorKind, Result};
use crate::id::Id;
use crate::pack::iter::PackfileIterator;
use crate::stores::{Queryable, StorageSet};

pub fn write<R, W, S>(
    mut input: R,
    output: &mut W,
    storage_set: Option<&StorageSet<S>>,
) -> Result<()>
where
    R: BufRead + Seek + Clone + Debug + Sync,
    W: Write,
    S: Queryable + Sync,
{
    let len = input.seek(SeekFrom::End(0))?;
    input.seek(SeekFrom::Start(0))?;

    let iter =
        PackfileIterator::new(input.clone(), storage_set).expect("failed to parse as packfile");
    let mut offsets = Vec::with_capacity(4096);

    // first pass: find all offsets and non-delta'd ids
    let objects: Vec<_> = iter
        .map(|(offset, pf_type, id)| {
            offsets.push(offset.clone());
            (offset, pf_type, id)
        })
        .collect();
    offsets.push(len - 20);

    // second pass: calculate crcs between offsets
    let windows: Vec<_> = offsets.windows(2).collect();
    let crcs: Vec<_> = windows
        .par_iter()
        .filter_map(|offset| {
            let mut digest = CRCDigest::new(crc32::IEEE);

            let mut cursor = input.clone();
            cursor.seek(SeekFrom::Start(offset[0])).ok()?;
            let mut input_bytes = Vec::with_capacity((offset[1] - offset[0]) as usize);
            cursor
                .take(offset[1] - offset[0])
                .read_to_end(&mut input_bytes)
                .ok()?;
            digest.write(&input_bytes);
            Some(digest.sum32())
        })
        .collect();

    if crcs.len() != objects.len() {
        return Err(ErrorKind::CorruptedPackfile.into());
    }

    // third pass: calculate delta reprs
    let mut decompressed: Vec<_> = objects
        .into_par_iter()
        .enumerate()
        .filter_map(|(idx, (offset, pf_type, id))| {
            if id.is_some() {
                return Some((idx, offset, id.unwrap()));
            }

            let mut input = input.clone();
            let mut output = Vec::new();
            let object_type = pf_type
                .decompress(offset, &mut input, &mut output, storage_set)
                .ok()?;
            let mut hash = Sha1::new();
            let header = format!("{} {}\0", object_type.as_str(), output.len());
            hash.input(header.as_bytes());
            hash.input(&output[..]);
            let mut id_output = [0u8; 20];
            hash.result(&mut id_output);
            Some((idx, offset, Id::from(&id_output[..])))
        })
        .collect();

    // sort the results by id hash (instead of offset order)
    decompressed.par_sort_unstable_by(|lhs, rhs| lhs.2.cmp(&rhs.2));

    let mut fanout = [0u32; 256]; // each value in fanout holds the upper bound index of the object starting with the incoming byte
    let mut byte = 0u8;
    fanout[0xff] = (decompressed.len() as u32).to_be();

    let mut offsets = Vec::with_capacity(decompressed.len());
    let mut large_offsets = Vec::new();
    let mut crcs_out = Vec::with_capacity(decompressed.len());
    let mut ids = Vec::with_capacity(decompressed.len());
    for (idx, (crc_idx, offset, id)) in decompressed.into_iter().enumerate() {
        if byte != id.as_ref()[0] {
            fanout[byte as usize] = (idx as u32).to_be();
            byte += 1;
        }

        ids.push(id);

        if offset > 0x7fff_ffff {
            offsets.push((large_offsets.len() as u32 & 0x8000_0000).to_be());
            large_offsets.push(offset.to_be());
        } else {
            offsets.push((offset as u32).to_be());
        }

        crcs_out.push(crcs[crc_idx].to_be());
    }

    let mut shasum = Sha1::new();

    let magic_byte = b"\xfftOc";
    shasum.input(magic_byte);
    output.write(magic_byte)?;

    let version_bytes = unsafe { std::mem::transmute::<u32, [u8; 4]>(2u32.to_be()) };
    shasum.input(&version_bytes);
    output.write(&version_bytes)?;

    let fanout_bytes = unsafe { std::mem::transmute::<[u32; 256], [u8; 256 * 4]>(fanout) };
    shasum.input(&fanout_bytes);
    output.write(&fanout_bytes)?;

    for id in ids {
        let id_bytes = id.as_ref();
        shasum.input(id_bytes);
        output.write(id_bytes)?;
    }

    for crc in crcs_out {
        let crc_bytes = unsafe { std::mem::transmute::<u32, [u8; 4]>(crc) };
        shasum.input(&crc_bytes);
        output.write(&crc_bytes)?;
    }

    for offset in offsets {
        let offset_bytes = unsafe { std::mem::transmute::<u32, [u8; 4]>(offset) };
        shasum.input(&offset_bytes);
        output.write(&offset_bytes)?;
    }

    for large_offset in large_offsets {
        let large_offset_bytes = unsafe { std::mem::transmute::<u64, [u8; 8]>(large_offset) };
        shasum.input(&large_offset_bytes);
        output.write(&large_offset_bytes)?;
    }

    input.seek(SeekFrom::End(-20))?;
    let mut packfile_checksum_bytes = Vec::with_capacity(20);

    input.read_to_end(&mut packfile_checksum_bytes)?;
    shasum.input(&packfile_checksum_bytes);
    output.write(&packfile_checksum_bytes)?;

    let mut checksum = [0u8; 20];
    shasum.result(&mut checksum);
    output.write(&checksum)?;

    Ok(())
}

pub fn read<R: Read>(mut input: R) -> Result<Index> {
    let mut magic = [0u8; 4];
    input.read_exact(&mut magic)?;
    let mut version = [0u8; 4];
    input.read_exact(&mut version)?;

    if (&magic != b"\xfftOc") {
        return Err(ErrorKind::InvalidPackfileIndex.into());
    }

    if (version != unsafe { std::mem::transmute::<u32, [u8; 4]>(2u32.to_be()) }) {
        return Err(ErrorKind::UnsupportedPackfileIndexVersion.into());
    }

    let mut fanout = [0u32; 256];
    input.read_u32_into::<BigEndian>(&mut fanout)?;

    let object_count = fanout[255] as usize;

    let mut oid_bytes_vec = vec![0u8; object_count * 20];
    input.read_exact(&mut oid_bytes_vec.as_mut_slice())?;

    let ids: Vec<Id> = oid_bytes_vec
        .chunks(20)
        .map(|chunk| Id::from(chunk))
        .collect();

    let mut crc_vec = vec![0u32; object_count];
    input.read_u32_into::<BigEndian>(&mut crc_vec.as_mut_slice())?;

    let mut offsets_vec = vec![0u32; object_count];
    input.read_u32_into::<BigEndian>(&mut offsets_vec.as_mut_slice())?;

    let mut large_offset_count = 0;
    for offset in offsets_vec.iter() {
        let msb_set = offset & 0x8000_0000;
        if msb_set > 0 {
            large_offset_count += 1;
        }
    }

    let mut large_offsets_vec = vec![0u64; large_offset_count as usize];
    input.read_u64_into::<BigEndian>(&mut large_offsets_vec)?;

    let offsets: Vec<_> = offsets_vec
        .into_iter()
        .map(|offset| {
            if offset & 0x8000_0000 != 0 {
                large_offsets_vec[(offset & 0x7FFF_FFFF) as usize]
            } else {
                offset as u64
            }
        })
        .collect();

    let mut offset_idx_sorted: Vec<(usize, &u64)> = offsets.iter().enumerate().collect();
    offset_idx_sorted.sort_by_key(|(_, offset)| *offset);

    let mut next_offsets_indices = vec![0; offset_idx_sorted.len()];
    let mut idx = 0;
    while idx < offset_idx_sorted.len() - 1 {
        next_offsets_indices[offset_idx_sorted[idx].0] = offset_idx_sorted[idx + 1].0;
        idx += 1;
    }

    Ok(Index {
        fanout,
        ids,
        offsets,
        next_offsets_indices,
        crcs: crc_vec,
    })
}

pub struct Index {
    fanout: [u32; 256],
    ids: Vec<Id>,
    offsets: Vec<u64>,
    next_offsets_indices: Vec<usize>,
    crcs: Vec<u32>,
}

impl Index {
    pub fn get_bounds(&self, id: &Id) -> Option<(u64, u64)> {
        let as_bytes: &[u8] = id.as_ref();
        let mut lo = if as_bytes[0] > 0 {
            self.fanout[(as_bytes[0] - 1) as usize]
        } else {
            0
        };
        let mut hi = self.fanout[as_bytes[0] as usize];
        let mut middle: usize;
        let len = self.offsets.len();
        loop {
            middle = ((lo + hi) >> 1) as usize;
            if middle >= len {
                return None;
            }

            match id.partial_cmp(&self.ids[middle]) {
                Some(xs) => match xs {
                    std::cmp::Ordering::Less => {
                        hi = middle as u32;
                    }
                    std::cmp::Ordering::Greater => {
                        lo = (middle + 1) as u32;
                    }
                    std::cmp::Ordering::Equal => {
                        return Some((
                            self.offsets[middle],
                            self.offsets[self.next_offsets_indices[middle]],
                        ));
                    }
                },
                None => return None,
            }

            if lo >= hi {
                break;
            }
        }

        None
    }
}
