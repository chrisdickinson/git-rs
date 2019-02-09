extern crate git_rs;

use std::io::{ self, Write, Cursor, BufReader };
use std::fs::File;
use memmap::MmapOptions;
use crypto::{ sha1::Sha1, digest::Digest };
use crc::crc32::Digest as CRCDigest;
use crc::crc32;
use rayon::prelude::*;
use crc::crc32::Hasher32;
use rayon;

use git_rs::stores::{fs as gitfs, StorageSet};
use git_rs::pack::iter::PackfileIterator;
use git_rs::errors::Result as GitResult;
use git_rs::id::Id;

pub fn main() -> std::io::Result<()> {
    let current_dir = std::env::current_dir()?;
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("must provide a filename");
        return Ok(())
    }

    let f = File::open(&args[1])?;
    let mmap = unsafe { MmapOptions::new().map(&f)? };

    let cursor = Cursor::new(&mmap[..]);

    // first pass: undelta'd objects
    let storage_set = gitfs::from(current_dir.as_path()).expect("failed to open storage");
    let iter = PackfileIterator::new(cursor, Some(&storage_set)).expect("failed to parse as packfile");
    let mut offsets = Vec::with_capacity(4096);

    let objects: Vec<_> = iter.map(|(offset, pf_type, id)| {
        offsets.push(offset.clone());
        (offset, pf_type, id)
    }).collect();

    offsets.push(mmap.len() as u64 - 20);

    // second pass: crcs
    let windows: Vec<_> = offsets.windows(2).collect();
    let crcs: Vec<_> = windows.par_iter().map(|offset| {
        let mut digest = CRCDigest::new(crc32::IEEE);
        digest.write(&mmap[offset[0] as usize .. offset[1] as usize]);
        digest.sum32()
    }).collect();

    //eprintln!("found offsets for {} elements...", objects.len());
    let mut decompressed: Vec<_> = objects
                        .into_par_iter()
                        .enumerate()
                        .filter_map(|(idx, (offset, pf_type, id))| {
        if id.is_some() {
            return Some((idx, offset, id.unwrap()))
        }

        let mut input = Cursor::new(&mmap[..]);
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

    let mut stdout = io::stdout();
    let mut shasum = Sha1::new();

    let magic_byte = b"\xfftOc";
    shasum.input(magic_byte);
    stdout.write(magic_byte)?;

    let version_bytes = unsafe { std::mem::transmute::<u32, [u8; 4]>(2u32.to_be()) };
    shasum.input(&version_bytes);
    stdout.write(&version_bytes)?;

    let fanout_bytes = unsafe { std::mem::transmute::<[u32; 256], [u8; 256 * 4]>(fanout) };
    shasum.input(&fanout_bytes);
    stdout.write(&fanout_bytes)?;

    for id in ids {
        let id_bytes = id.as_ref();
        shasum.input(id_bytes);
        stdout.write(id_bytes)?;
    }

    for crc in crcs_out {
        let crc_bytes = unsafe { std::mem::transmute::<u32, [u8; 4]>(crc) };
        shasum.input(&crc_bytes);
        stdout.write(&crc_bytes)?;
    }

    for offset in offsets {
        let offset_bytes = unsafe { std::mem::transmute::<u32, [u8; 4]>(offset) };
        shasum.input(&offset_bytes);
        stdout.write(&offset_bytes)?;
    }

    for large_offset in large_offsets {
        let large_offset_bytes = unsafe { std::mem::transmute::<u64, [u8; 8]>(large_offset) };
        shasum.input(&large_offset_bytes);
        stdout.write(&large_offset_bytes)?;
    }

    let packfile_checksum_bytes = &mmap[mmap.len() - 20 ..];
    shasum.input(&packfile_checksum_bytes);
    stdout.write(&packfile_checksum_bytes)?;

    let mut checksum = [0u8; 20];
    shasum.result(&mut checksum);
    stdout.write(&checksum)?;

    Ok(())
}
