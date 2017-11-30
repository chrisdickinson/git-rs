use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::vec::Vec;
use glob::glob;

use id::Id;
use reference::Ref;
use error::GitError;
use stores::{Queryable, loose};
use objects::GitObject;

#[derive(Debug)]
pub struct Repository {
    path: PathBuf,
    heads: HashMap<String, Ref>,
    stores: Vec<Box<Queryable>>,
}

impl Repository {
    pub fn from_fs(path: &Path) -> Repository {
        let mut heads = HashMap::new();
        let pb = PathBuf::from(path);

        let mut glob_path = pb.clone();
        glob_path.push("refs");
        glob_path.push("heads");
        glob_path.push("*");

        if let Some(glob_path_str) = glob_path.to_str() {
            for entry in glob(glob_path_str).expect("Weena wonga") {
                let item = match entry {
                    Ok(item) => item,
                    Err(_e) => continue,
                };
                if let Some(item_as_str) = item.to_str() {
                    let name = item_as_str.replace(pb.to_str().unwrap(), "").replace(
                        "/refs/heads/",
                        "",
                    );

                    if let Ok(reference) = Ref::new(item_as_str) {

                        heads.insert(name, reference);
                    }
                }
            }
        }

        let mut repository = Repository {
            path: pb.clone(),
            heads: heads,
            stores: Vec::new(),
        };
        repository.stores.push(Box::new(loose::Store::new()));
        repository
    }

    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    pub fn get_object(&self, id: &Id) -> Result<Option<GitObject>, GitError> {
        for store in &self.stores {
            let result = match store.get(self, id) {
                Ok(v) => v,
                Err(err) => return Err(err),
            };

            if let Some(obj) = result {
                return Ok(Some(obj));
            }
        }
        return Ok(None);
    }
}
