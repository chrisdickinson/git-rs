use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::Read;

use crate::id::Id;

#[derive(Copy, Clone, Debug)]
pub enum Kind {
    Local,
    Remote,
    Tag
}

#[derive(Debug)]
pub enum RefPtr {
    Indirect(String),
    Direct(Id)
}

#[derive(Debug)]
pub struct Ref {
    kind: Kind,
    ptr: RefPtr
}

pub struct RefSet(HashMap<String, Ref>);

impl Ref {
    pub fn load(path: &Path, kind: Kind) -> Result<Ref, std::io::Error> {
        let mut f = File::open(path)?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)?;

        if let Ok(contents) = std::str::from_utf8(&buffer) {
            if contents.len() < 5 {
                return Err(std::io::ErrorKind::InvalidData.into());
            }

            if &contents[0..16] == "ref: refs/heads/" {
                return Ok(Ref {
                    kind,
                    ptr: RefPtr::Indirect(String::from(contents[16..].trim()))
                });
            }

            if buffer.len() >= 40 {
                if let Some(id) = Id::from_str(&contents[0..40]) {
                    return Ok(Ref {
                        ptr: RefPtr::Direct(id),
                        kind
                    });
                }
            }
        }

        return Err(std::io::ErrorKind::InvalidData.into());
    }
}

fn recurse_dir<'a>(
    root: &mut PathBuf,
    dirs: &mut Vec<String>,
    map: &mut HashMap<String, Ref>,
    k: Kind
) -> Result<(), std::io::Error> {
    for entry in std::fs::read_dir(root.as_path())? {
        let entry = entry?;

        let typ = entry.file_type()?;
        let os_filename = entry.file_name();
        let opt_filename = os_filename.to_str();
        if opt_filename.is_none() {
            continue
        }
        let filename = opt_filename.unwrap();

        dirs.push(String::from(filename));
        if typ.is_dir() {
            recurse_dir(&mut entry.path(), dirs, map, k)?;
        } else {
            let ref_name: String = dirs.join("/");
            if let Ok(reference) = Ref::load(&entry.path(), k) {
                map.insert(ref_name, reference);
            }
        }
        dirs.pop();
    }

    Ok(())
}

impl RefSet {
    pub fn from_path(path: &Path) -> Result<RefSet, std::io::Error> {
        let mut root = std::path::PathBuf::new();
        let mut map = HashMap::new();
        let mut dirs = Vec::new();
        root.push(path);
        root.push(".git");
        root.push("refs");
        root.push("heads");
        recurse_dir(&mut root, &mut dirs, &mut map, Kind::Local)?;
        root.pop();
        root.push("remotes");
        recurse_dir(&mut root, &mut dirs, &mut map, Kind::Remote)?;
        root.pop();
        root.push("tags");
        recurse_dir(&mut root, &mut dirs, &mut map, Kind::Tag)?;
        root.pop();
        root.pop();
        root.push("HEAD");
        if let Ok(reference) = Ref::load(root.as_path(), Kind::Local) {
            map.insert(String::from("HEAD"), reference);
        };

        Ok(RefSet {
            0: map
        })
    }

    pub fn deref(&self, name: &str) -> Option<&Id> {
        let mut reference = self.0.get(name);
        loop {
            match reference {
                Some(xs) => {
                    match xs.ptr {
                        RefPtr::Direct(ref id) => return Some(&id),
                        RefPtr::Indirect(ref string) => {
                            reference = self.0.get(string.as_str());
                        }
                    }
                },
                None => return None
            }
        }
    }
}
