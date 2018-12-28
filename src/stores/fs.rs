use crate::stores::pack::{ Store as PackStore, GetObject };
use crate::stores::loose::{ Store as LooseStore };
use crate::stores::{ Storage, StorageSet };
use crate::packindex::Index;

use std::path::Path;

pub fn from(path: &Path) -> Result<StorageSet, std::io::Error> {
    let mut backends = Vec::new();
    loose_from_path(path, &mut backends);
    packfiles_from_path(path, &mut backends)?;
    Ok(StorageSet::new(backends))
}

pub fn loose_from_path(path: &Path, stores: &mut Vec<Box<Storage>>) {
    let mut root = std::path::PathBuf::new();
    root.push(path);
    root.push(".git");
    root.push("objects");

    let loose_store = LooseStore::new(move |id| {
        let as_str = id.to_string();
        let mut pb = root.clone();
        pb.push(as_str[0..2].to_string());
        pb.push(as_str[2..40].to_string());
        match std::fs::File::open(pb.as_path()) {
            Ok(f) => Ok(Some(Box::new(f))),
            Err(e) => {
                match e.kind() {
                    std::io::ErrorKind::NotFound => return Ok(None),
                    _ => return Err(e)?
                }
            }
        }
    });

    stores.push(Box::new(loose_store));
}

pub fn packfiles_from_path(path: &Path, stores: &mut Vec<Box<Storage>>) -> Result<(), std::io::Error> {
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

        let store = PackStore::new(move || {
            Ok(std::fs::File::open(epb.as_path()).expect("success?"))
        }, Some(idx));

        if let Ok(store) = store {
            stores.push(Box::new(store));
        }
    }

    Ok(())
}
