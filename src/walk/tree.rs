use std::collections::btree_map::IntoIter;
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;

use crate::objects::blob::Blob;
use crate::objects::tree::{FileMode, TreeEntry};
use crate::objects::Object;
use crate::stores::{Queryable, StorageSet};

pub struct TreeIterator<'a, S: Queryable> {
    storage_set: &'a StorageSet<S>,
    layers: Vec<IntoIter<Vec<u8>, TreeEntry>>,
    path_segments: PathBuf,
}

impl<'a, S: Queryable> TreeIterator<'a, S> {
    pub fn new(
        storage_set: &'a StorageSet<S>,
        layers: Vec<IntoIter<Vec<u8>, TreeEntry>>,
    ) -> TreeIterator<S> {
        TreeIterator {
            path_segments: PathBuf::from("."),
            storage_set,
            layers,
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
                continue;
            }

            let (key, entry) = next.unwrap();
            let item = self
                .storage_set
                .get_and_load(&entry.id)
                .ok()
                .unwrap_or(None);
            if let Some(xs) = item {
                match xs {
                    Object::Tree(xs) => {
                        self.path_segments.push(OsStr::from_bytes(&key));
                        self.layers.push(xs.into_iter());
                    }

                    Object::Blob(xs) => {
                        let mut pb = self.path_segments.clone();
                        pb.push(OsStr::from_bytes(&key));
                        return Some((pb, entry.mode, xs));
                    }

                    _ => continue,
                }
            }
        }
    }
}
