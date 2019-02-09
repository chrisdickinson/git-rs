use crate::stores::loose::{ Store as LooseStore };
use crate::pack::mmap::Reader as MmapPackReader;
use crate::stores::pack::{ Store as PackStore };
use crate::stores::StorageSet;
use crate::packindex::Index;
use memmap::MmapOptions;

use std::path::Path;

pub fn from(path: &Path) -> Result<StorageSet<(Vec<PackStore<MmapPackReader>>, LooseStore)>, std::io::Error> {
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
        pb.push(as_str[0..2].to_string());
        pb.push(as_str[2..40].to_string());
        match std::fs::File::open(pb.as_path()) {
            Ok(f) => Ok(Some(Box::new(f))),
            Err(e) => {
                match e.kind() {
                    std::io::ErrorKind::NotFound => Ok(None),
                    _ => Err(e)?
                }
            }
        }
    }, Some(filter));

    Ok(loose_store)
}

pub fn packfiles_from_path(path: &Path) -> Result<Vec<PackStore<MmapPackReader>>, std::io::Error> {
    let mut stores = vec![];
    let mut root = std::path::PathBuf::new();
    root.push(path);
    root.push(".git");
    root.push("objects");
    root.push("pack");

    for entry in std::fs::read_dir(root.as_path())? {
        let entry = entry?;
        let os_filename = entry.file_name();
        let filename = os_filename.to_str();
        if filename.is_none() {
            continue
        }

        if !filename.unwrap().ends_with(".idx") {
            continue
        }

        let entry_path = entry.path();
        let idx = match Index::from(std::fs::File::open(entry_path.clone())?) {
            Ok(xs) => xs,
            Err(_) => return Err(std::io::ErrorKind::InvalidData.into())
        };

        let mut epb = entry_path.to_path_buf();
        epb.set_extension("pack");


        let file = std::fs::File::open(epb.as_path())?;
        let mmap = unsafe { MmapOptions::new().map(&file)? };
        let packfile = MmapPackReader::new(mmap);
        stores.push(PackStore::new(packfile, idx));
    }

    Ok(stores)
}
