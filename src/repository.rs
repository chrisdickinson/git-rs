use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::vec::Vec;
use glob::glob;

use id::Id;
use reference::Ref;
use error::GitError;
use objects::Type;
use objects::commit::Commit;
use stores::{Queryable, loose, pack};

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

        let mut packed_refs_path = pb.clone();
        packed_refs_path.push("packed-refs");

        Ref::from_packed(packed_refs_path.to_str().unwrap(), &mut heads);

        let mut glob_refs_path = pb.clone();
        glob_refs_path.push("refs");
        glob_refs_path.push("heads");
        glob_refs_path.push("*");

        if let Some(glob_refs_path_str) = glob_refs_path.to_str() {
            for entry in glob(glob_refs_path_str).expect("Weena wonga") {
                let item = match entry {
                    Ok(item) => item,
                    Err(_e) => continue,
                };
                if let Some(item_as_str) = item.to_str() {
                    let name = item_as_str.replace(pb.to_str().unwrap(), "").replace(
                        "/refs/",
                        "/",
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

        let mut glob_packs_path = pb.clone();
        glob_packs_path.push("objects");
        glob_packs_path.push("pack");
        glob_packs_path.push("*.idx");

        if let Some(glob_packs_path_str) = glob_packs_path.to_str() {
            for entry in glob(glob_packs_path_str).expect("Weena wonga") {
                let item = match entry {
                    Ok(item) => item,
                    Err(_e) => continue,
                };

                if let Some(item_as_str) = item.to_str() {
                    let pack_idx_path = PathBuf::from(item_as_str);
                    let mut pack_path = pack_idx_path.clone();
                    pack_path.set_extension("pack");

                    let store = match pack::Store::new(
                        &pack_idx_path,
                        &pack_path
                    ) {
                        Ok(xs) => xs,
                        Err(_) => continue
                    };

                    repository.stores.push(Box::new(store));
                }
            }
        }

        repository
    }

    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    pub fn rev_parse (&self, what: &str) -> Option<Id> {
        if let Ok(id) = Id::from(what) {
            return Some(id)
        }

        if let Some(head) = self.heads.get(what) {
            return Some(head.to_id())
        }

        return None
    }

    pub fn lookup (&self, what: &str) -> Result<Option<Type>, GitError> {
        if let Some(id) = self.rev_parse(what) {
            return self.get_object(&id)
        }
        Ok(None)
    }

    pub fn get_object(&self, id: &Id) -> Result<Option<Type>, GitError> {
        for store in &self.stores {
            let xs = store.get(self, id);
            let result = match xs {
                Ok(v) => v,
                Err(err) => return Err(err),
            };

            if let Some(obj) = result {
                return Ok(Some(obj));
            }
        }
        return Ok(None);
    }

    pub fn get_path_at_commit(&self, what: &Commit, path: Vec<&str>) -> Result<Option<Type>, GitError> {
        Ok(None)
    }
}
