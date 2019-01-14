use std::collections::hash_map::{ Iter, IntoIter };
use std::path::{ Path, PathBuf };
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

use crate::objects::tree::{ Tree, TreeEntry, FileMode };
use crate::objects::blob::Blob;
use crate::stores::StorageSet;
use crate::objects::Object;
use crate::id::Id;

pub struct TreeIterator<'a> {
    storage_set: &'a StorageSet,
    layers: Vec<IntoIter<Vec<u8>, TreeEntry>>,
    path_segments: PathBuf
}

impl<'a> Iterator for TreeIterator<'a> {
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

            let (mut key, entry) = next.unwrap();
            let item = self.storage_set.get_and_load(&entry.id).ok().unwrap_or(None);
            if let Some(xs) = item {
                match xs {
                    Object::Tree(xs) => {
                        self.path_segments.push(OsStr::from_bytes(&key));
                        self.layers.push(xs.into_iter());
                    },

                    Object::Blob(xs) => {
                        return Some(
                            (self.path_segments.clone(), entry.mode, xs)
                        )
                    },

                    _ => continue
                }
            }
        }

        None
    }
}
