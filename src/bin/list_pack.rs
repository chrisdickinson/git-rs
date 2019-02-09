extern crate git_rs;

use std::fs::File;
use std::io::Cursor;
use std::io::BufReader;
use memmap::MmapOptions;
use crypto::{ sha1::Sha1, digest::Digest };

use git_rs::stores::{fs as gitfs, StorageSet};
use git_rs::pack::iter::PackfileIterator;
use git_rs::errors::Result as GitResult;
use git_rs::id::Id;

use crc::crc32::Digest as CRCDigest;
use crc::crc32;
use rayon::prelude::*;
use crc::crc32::Hasher32;

pub fn main() -> std::io::Result<()> {
    let current_dir = std::env::current_dir()?;
    let storage_set = gitfs::from(current_dir.as_path())?;
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("must provide a filename");
        return Ok(())
    }

    let f = File::open(&args[1])?;
    let mmap = unsafe { MmapOptions::new().map(&f)? };
    let cursor = Cursor::new(&mmap[..]);


    // first pass: undelta'd objects
    let iter = PackfileIterator::new(cursor, Some(&storage_set)).expect("failed to parse as packfile");
    let mut offsets = Vec::with_capacity(4096);

    let (bases, deltas) : (Vec<_>, Vec<_>) = iter.partition(|(offset, pf_type, id)| {
        offsets.push(offset.clone());
        id.is_some()
    });

    offsets.push(mmap.len() as u64);

    // second pass: crcs
    let windows: Vec<_> = offsets.windows(2).collect();
    let crcs: Vec<_> = windows.par_iter().map(|offset| {
        let mut digest = CRCDigest::new(crc32::IEEE);
        digest.write(&mmap[offset[0] as usize .. offset[1] as usize]);
        (offset[0], digest.sum32())
    }).collect();

    println!("crcs={:?}; bases.len()={}; deltas={}", crcs.len(), bases.len(), deltas.len());

    // now we want to calculate deltas in parallel. this is tough because they
    // depend on each other.
    let decompressed: Vec<GitResult<_>> = deltas.into_par_iter().map(|(offset, pf_type, id)| {
        let mut input = Cursor::new(&mmap[..]);
        let mut output = Vec::new();
        let object_type = pf_type.decompress(
            offset,
            &mut input,
            &mut output,
            Some(&StorageSet::new(()))
        )?;
        let mut hash = Sha1::new();
        let header = format!("{} {}\0", object_type.as_str(), output.len());
        hash.input(header.as_bytes());
        hash.input(&output[..]);
        let mut id_output = [0u8; 20];
        hash.result(&mut id_output);
        Ok((offset, Id::from(&id_output[..])))
    }).collect();

    Ok(())
}
