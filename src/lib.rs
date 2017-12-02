extern crate multimap;
extern crate flate2;
extern crate glob;
extern crate hex;

mod repository;
mod reference;
mod objects;
mod stores;
mod error;
mod id;

use std::path::{Path, PathBuf};
use repository::Repository;
use objects::GitObject;
use id::Id;

#[cfg(test)]
mod tests {
    use Repository;
    use Path;
    use Id;
    use GitObject;

    #[test]
    fn it_works() {
        let repo =
            Repository::from_fs(Path::new("/Users/chris/projects/personal/tempisfugit/.git"));

        println!("repo: {:?}", repo);
    }

    #[test]
    fn it_works_pt_deux() {
        let repo =
            Repository::from_fs(Path::new("/Users/chris/projects/personal/git-rs/.git"));

        let mut id = Id::from("89012d389533666964c4a0032249796c3a3b3ba6");
        loop {
            if let Ok(Some(result)) = repo.get_object(&id) {
                if let GitObject::CommitObject(commit) = result {
                    println!("{:?} {}", id, commit.message());
                    let parents = match commit.parents(&repo) {
                        Some(v) => v,
                        None => break
                    };
                    if let Some(parent) = parents.first() {
                        id = Id::from(parent.as_str());
                    } else { break }
                } else {
                    break
                }
            } else {
                break
            }
        }
    }
}
