use std::{collections::btree_map::{ IntoIter }, ffi::OsString};
use std::path::{ PathBuf };
use std::ffi::OsStr;
use crate::objects::tree::{ TreeEntry, FileMode };
use crate::stores::{ StorageSet, Queryable };
use crate::objects::blob::Blob;
use crate::objects::Object;

pub struct TreeIterator<'a, S: Queryable> {
    storage_set: &'a StorageSet<S>,
    layers: Vec<IntoIter<Vec<u8>, TreeEntry>>,
    path_segments: PathBuf
}

impl<'a, S: Queryable> TreeIterator<'a, S> {
    pub fn new(storage_set: &'a StorageSet<S>, layers: Vec<IntoIter<Vec<u8>, TreeEntry>>) -> TreeIterator<S> {
        TreeIterator {
            path_segments: PathBuf::from("."),
            storage_set,
            layers
        }
    }
}

impl<'a, S: Queryable> Iterator for TreeIterator<'a, S> {
    type Item = (PathBuf, FileMode, Blob);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let current_iter = self.layers.last_mut()?;
            let next = current_iter.next();
            if next.is_none() {
                self.layers.pop();
                self.path_segments.pop();
                continue
            }

            let (key, entry) = next.unwrap();
            let item = self.storage_set.get_and_load(&entry.id).ok().unwrap_or(None);
            if let Some(xs) = item {
                match xs {
                    Object::Tree(xs) => {
                        self.path_segments.push(OsStr::from_byte(&key).as_os_str());
                        self.layers.push(xs.into_iter());
                    },

                    Object::Blob(xs) => {
                        let mut pb = self.path_segments.clone();
                        pb.push(OsStr::from_byte(&key).as_os_str());
                        return Some(
                            (pb, entry.mode, xs)
                        )
                    },

                    _ => continue
                }
            }
        }
    }
}

trait FromBytes {
    fn from_byte(bytes: &[u8]) -> OsString;
}

impl FromBytes for OsStr {
    fn from_byte(bytes: &[u8]) -> OsString {
        #[cfg(unix)] {
            use std::os::unix::ffi::OsStrExt;
            OsStr::from_bytes(bytes).to_os_string()
        }
        #[cfg(windows)] {
            use std::os::windows::prelude::*;
            OsString::from_wide(bytes.iter().map(|x| *x as u16).collect::<Vec<u16>>().as_slice())
        }
    }
}