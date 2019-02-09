use crc::crc32::{ self, Digest as CRCDigest, Hasher32 };
use crypto::{ sha1::Sha1, digest::Digest };
use std::io::{ Cursor, SeekFrom };
use std::io::prelude::*;
use rayon::prelude::*;
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
    S: Queryable {

    let len = input.seek(SeekFrom::End(0))?;
    input.seek(SeekFrom::Start(0))?;

    let iter = PackfileIterator::new(input.clone(), storage_set).expect("failed to parse as packfile");
    let mut offsets = Vec::with_capacity(4096);

    // first pass: find all offsets and non-delta'd ids
    let objects: Vec<_> = iter.map(|(offset, pf_type, id)| {
        offsets.push(offset.clone());
        (offset, pf_type, id)
    }).collect();
    offsets.push(len - 20);

    // second pass: calculate crcs between offsets
    let windows: Vec<_> = offsets.windows(2).collect();
    let crcs: Vec<_> = windows.par_iter().filter_map(|offset| {
        let mut digest = CRCDigest::new(crc32::IEEE);

        let mut cursor = input.clone();
        cursor.seek(SeekFrom::Start(offset[0])).ok()?;
        let mut input_bytes = Vec::with_capacity((offset[1] - offset[0]) as usize);
        cursor.take(offset[1] - offset[0]).read_to_end(&mut input_bytes).ok()?;
        digest.write(&input_bytes);
        Some(digest.sum32())
    }).collect();

    if crcs.len() != objects.len() {
        return Err(ErrorKind::CorruptedPackfile.into());
    }

    // third pass: calculate delta reprs
    let mut decompressed: Vec<_> = objects
                        .into_par_iter()
                        .enumerate()
                        .filter_map(|(idx, (offset, pf_type, id))| {
        if id.is_some() {
            return Some((idx, offset, id.unwrap()))
        }

        let mut input = input.clone();
        let mut output = Vec::new();
        let object_type = pf_type.decompress(
            offset,
            &mut input,
            &mut output,
            Some(&StorageSet::new(()))
        ).ok()?;
        let mut hash = Sha1::new();
        let header = format!("{} {}\0", object_type.as_str(), output.len());
        hash.input(header.as_bytes());
        hash.input(&output[..]);
        let mut id_output = [0u8; 20];
        hash.result(&mut id_output);
        Some((idx, offset, Id::from(&id_output[..])))
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
