use crc::{ Crc, CRC_32_ISO_HDLC };
use crypto::{ sha1::Sha1, digest::Digest };
use byteorder::{ BigEndian, ReadBytesExt };
use std::io::prelude::*;
use rayon::prelude::*;
use std::io::SeekFrom;
use std::fmt::Debug;

use crate::stores::{ StorageSet, Queryable };
use crate::errors::{ ErrorKind, Result };
use crate::pack::iter::PackfileIterator;
use crate::id::Id;

pub fn write<R, W, S>(
    mut input: R,
    output: &mut W,
    storage_set: Option<&StorageSet<S>>
) -> Result<()> where
    R: BufRead + Seek + Clone + Debug + Sync,
    W: Write,
    S: Queryable + Sync {

    let len = input.seek(SeekFrom::End(0))?;
    input.seek(SeekFrom::Start(0))?;

    let iter = PackfileIterator::new(input.clone())?;
    let mut offsets = Vec::with_capacity(4096);

    // first pass: find all offsets and non-delta'd ids
    let objects: Vec<_> = iter.map(|(offset, pf_type, id)| {
        offsets.push(offset);
        (offset, pf_type, id)
    }).collect();
    offsets.push(len - 20);

    // second pass: calculate crcs between offsets
    let windows: Vec<_> = offsets.windows(2).collect();
    let crcs: Vec<_> = windows.par_iter().filter_map(|offset| {
        let crc = Crc::<u32>::new(&CRC_32_ISO_HDLC);
        let mut digest = crc.digest();

        let mut cursor = input.clone();
        cursor.seek(SeekFrom::Start(offset[0])).ok()?;
        let mut input_bytes = Vec::with_capacity((offset[1] - offset[0]) as usize);
        cursor.take(offset[1] - offset[0]).read_to_end(&mut input_bytes).ok()?;
        digest.update(&input_bytes);
        Some(digest.finalize())
    }).collect();

    if crcs.len() != objects.len() {
        return Err(ErrorKind::CorruptedPackfile.into());
    }

    // third pass: calculate delta reprs
    let mut decompressed: Vec<_> = objects
                        .into_par_iter()
                        .enumerate()
                        .filter_map(|(idx, (offset, pf_type, id))| {
        if let Some(id) = id {
            return Some((idx, offset, id))
        }

        let mut input = input.clone();
        let mut output = Vec::new();
        let object_type = pf_type.decompress(
            offset,
            &mut input,
            &mut output,
            storage_set
        ).ok()?;
        let mut hash = Sha1::new();
        let header = format!("{} {}\0", object_type.as_str(), output.len());
        hash.input(header.as_bytes());
        hash.input(&output[..]);
        let mut id_output = [0u8; 20];
        hash.result(&mut id_output);
        Some((idx, offset, id_output.into()))
    }).collect();

    // sort the results by id hash (instead of offset order)
    decompressed.par_sort_unstable_by(|lhs, rhs| {
        lhs.2.cmp(&rhs.2)
    });

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
    output.write_all(magic_byte)?;

    let version_bytes = 2u32.to_be().to_ne_bytes();
    shasum.input(&version_bytes);
    output.write_all(&version_bytes)?;

    let fanout_bytes = unsafe { std::mem::transmute::<[u32; 256], [u8; 256 * 4]>(fanout) };
    shasum.input(&fanout_bytes);
    output.write_all(&fanout_bytes)?;

    for id in ids {
        let id_bytes = id.as_ref();
        shasum.input(id_bytes);
        output.write_all(id_bytes)?;
    }

    for crc in crcs_out {
        let crc_bytes = crc.to_be().to_ne_bytes();
        shasum.input(&crc_bytes);
        output.write_all(&crc_bytes)?;
    }

    for offset in offsets {
        let offset_bytes = offset.to_be().to_ne_bytes();
        shasum.input(&offset_bytes);
        output.write_all(&offset_bytes)?;
    }

    for large_offset in large_offsets {
        let large_offset_bytes = large_offset.to_be().to_ne_bytes();
        shasum.input(&large_offset_bytes);
        output.write_all(&large_offset_bytes)?;
    }

    input.seek(SeekFrom::End(-20))?;
    let mut packfile_checksum_bytes = Vec::with_capacity(20);

    input.read_to_end(&mut packfile_checksum_bytes)?;
    shasum.input(&packfile_checksum_bytes);
    output.write_all(&packfile_checksum_bytes)?;

    let mut checksum = [0u8; 20];
    shasum.result(&mut checksum);
    output.write_all(&checksum)?;

    Ok(())
}

pub fn read<R: Read>(mut input: R) -> Result<Index> {
    let mut magic = [0u8; 4];
    input.read_exact(&mut magic)?;
    let mut version = [0u8; 4];
    input.read_exact(&mut version)?;

    if &magic != b"\xfftOc" {
        return Err(ErrorKind::InvalidPackfileIndex.into())
    }

    if version != 2u32.to_be().to_ne_bytes() {
        return Err(ErrorKind::UnsupportedPackfileIndexVersion.into())
    }

    let mut fanout = [0u32; 256];
    input.read_u32_into::<BigEndian>(&mut fanout)?;

    let object_count = fanout[255] as usize;

    let ids = Id::read_packed_ids(&mut input, object_count)?;

    let mut crc_vec = vec!(0u32; object_count);
    input.read_u32_into::<BigEndian>(crc_vec.as_mut_slice())?;

    let mut offsets_vec = vec!(0u32; object_count);
    input.read_u32_into::<BigEndian>(offsets_vec.as_mut_slice())?;

    let mut large_offset_count = 0;
    for offset in offsets_vec.iter() {
        let msb_set = offset & 0x8000_0000;
        if msb_set > 0 {
            large_offset_count += 1;
        }
    }

    let mut large_offsets_vec = vec!(0u64; large_offset_count as usize);
    input.read_u64_into::<BigEndian>(&mut large_offsets_vec)?;

    let offsets: Vec<_> = offsets_vec.into_iter().map(|offset| {
        if offset & 0x8000_0000 != 0 {
            large_offsets_vec[(offset & 0x7FFF_FFFF) as usize]
        } else {
            offset as u64
        }
    }).collect();

    let mut offset_idx_sorted: Vec<(usize, &u64)> = offsets.iter().enumerate().collect();
    offset_idx_sorted.sort_unstable_by_key(|(_, offset)| *offset);

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
        crcs: crc_vec
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
    pub fn crcs(&self) -> &[u32] {
        &self.crcs
    }

    pub fn get_bounds(&self, id: &Id) -> Option<(u64, u64)> {
        let as_bytes: &[u8] = id.as_ref();
        let mut lo = if as_bytes[0] > 0 {
            self.fanout[(as_bytes[0] - 1) as usize]
        } else {
            0
        };
        let mut hi = self.fanout[as_bytes[0] as usize];
        let len = self.offsets.len();
        loop {
            let middle = ((lo + hi) >> 1) as usize;

            match id.partial_cmp(&self.ids[middle]) {
                Some(std::cmp::Ordering::Less) => { hi = middle as u32; },
                Some(std::cmp::Ordering::Greater) => { lo = (middle + 1) as u32; }
                Some(std::cmp::Ordering::Equal) => {
                    return Some((
                        self.offsets[middle],
                        self.offsets[self.next_offsets_indices[middle]]
                    ));
                },
                None => {
                    return None;
                }
            }

            if lo >= hi || middle >= len {
                return None
            }
        }
    }
}
