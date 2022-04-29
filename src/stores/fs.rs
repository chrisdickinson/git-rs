use crate::stores::loose::{ Store as LooseStore };
use crate::pack::index::{ read as read_packidx };
use crate::pack::mmap::Reader as MmapPackReader;
use crate::stores::pack::{ Store as PackStore };
use crate::stores::StorageSet;
use memmap::MmapOptions;
use rayon::prelude::*;

use std::path::Path;

type GitFSStore = (Vec<PackStore<MmapPackReader>>, LooseStore);

pub fn from(path: &Path) -> Result<StorageSet<GitFSStore>, std::io::Error> {
    let packfiles = packfiles_from_path(path)?;
    let loose = loose_from_path(path)?;

    Ok(StorageSet::new((
        packfiles,
        loose
    )))
}

pub fn loose_from_path(path: &Path) -> Result<LooseStore, std::io::Error> {
    let mut root = std::path::PathBuf::new();
    root.push(path);
    root.push(".git");
    root.push("objects");

    let mut filter = [false; 256];
    for entry in std::fs::read_dir(root.as_path())? {
        let entry = entry?;
        let os_filename = entry.file_name();
        if os_filename.len() != 2 {
            continue
        }

        let result = match usize::from_str_radix(&os_filename.to_string_lossy(), 16) {
            Ok(xs) => xs,
            Err(_) => continue
        };
        filter[result] = true;
    }

    let loose_store = LooseStore::new(move |id| {
        let as_str = id.to_string();
        let mut pb = root.clone();
        pb.push(&as_str[0..2]);
        pb.push(&as_str[2..40]);
        match std::fs::File::open(pb.as_path()) {
            Ok(f) => Ok(Some(Box::new(f))),
            Err(e) => {
                match e.kind() {
                    std::io::ErrorKind::NotFound => Ok(None),
                    _ => Err(e.into())
                }
            }
        }
    }, Some(filter));

    Ok(loose_store)
}

pub fn packfiles_from_path(path: &Path) -> Result<Vec<PackStore<MmapPackReader>>, std::io::Error> {
    let mut root = std::path::PathBuf::new();
    root.push(path);
    root.push(".git");
    root.push("objects");
    root.push("pack");

    let candidates: Vec<_> = std::fs::read_dir(root.as_path())?.filter_map(|entry| {
        let entry = entry.ok()?;
        let os_filename = entry.file_name();
        let filename = os_filename.to_str();

        if !filename?.ends_with(".idx") {
            return None
        }

        Some(entry)
    }).collect();

    let stores: Vec<PackStore<MmapPackReader>> = candidates.into_par_iter().map(|entry|  -> Result<PackStore<MmapPackReader>, std::io::Error> {
        let mut entry_path = entry.path();

        let index_file = std::fs::File::open(entry_path.clone())?;
        let index_mmap = unsafe { MmapOptions::new().map(&index_file)? };
        let idx = match read_packidx(std::io::Cursor::new(index_mmap)) {
            Ok(xs) => xs,
            Err(_) => return Err(std::io::ErrorKind::InvalidData.into())
        };

        entry_path.set_extension("pack");

        let file = std::fs::File::open(entry_path.as_path())?;
        let mmap = unsafe { MmapOptions::new().map(&file)? };
        let packfile = MmapPackReader::new(mmap);

        Ok(PackStore::new(packfile, idx))
    }).collect::<Result<Vec<_>, _>>()?;

    Ok(stores)
}
