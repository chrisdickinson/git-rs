#[macro_use] extern crate lazy_static;
extern crate byteorder;
extern crate multimap;
extern crate flate2;
extern crate regex;
extern crate glob;
extern crate hex;

mod repository;
mod packindex;
mod reference;
mod objects;
mod stores;
mod delta;
mod error;
mod id;

#[cfg(test)]
mod tests {
    use repository::Repository;
    use std::path::Path;
    use id::Id;
    use objects::GitObject;
    use std::io::Read;

    #[test]
    fn it_works() {
        let repo =
            Repository::from_fs(Path::new("/Users/chris/projects/personal/tempisfugit/.git"));

        // println!("repo: {:?}", repo);
    }

    #[test]
    fn commits() {
        let repo =
            Repository::from_fs(Path::new("/Users/chris/projects/personal/git-rs/.git"));

        let mut id = match repo.rev_parse("refs/heads/master") {
            Some(xs) => xs,
            None => return
        };
        loop {
            if let Ok(Some(result)) = repo.get_object(&id) {
                if let GitObject::CommitObject(commit) = result {
                    println!("commit {} {}", id, commit.message().trim());
                    let parents = match commit.parents() {
                        Some(v) => v,
                        None => {
                            println!("no gods, no parents");
                            return
                        }
                    };
                    if let Some(parent) = parents.first() {
                        id = Id::from(parent.as_str()).expect("failed to get ID");
                    } else {
                        return
                    }
                } else {
                    return
                }
            } else {
                return
            }
        }
    }

    #[test]
    fn commit_tree() {
        let repo =
            Repository::from_fs(Path::new("/Users/chris/projects/personal/git-rs/.git"));

        let id = match repo.rev_parse("refs/heads/master") {
            Some(xs) => xs,
            None => return
        };
        let result = match repo.get_object(&id) {
            Ok(xs) => match xs { Some(ys) => ys, None => return },
            Err(_) => return
        };
        let commit = match result {
            GitObject::CommitObject(xs) => xs,
            _ => return
        };
        let tree_id = match commit.tree() {
            Some(xs) => xs,
            None => return
        };

        let tree = repo.get_object(&Id::from(tree_id).expect("bad tree"));
        println!("tree: {:?}", tree);
    }

    #[test]
    fn commit_blob() {
        let repo =
            Repository::from_fs(Path::new("/Users/chris/projects/personal/git-rs/.git"));

        let id = match repo.rev_parse("refs/heads/master") {
            Some(xs) => xs,
            None => return
        };
        let result = match repo.get_object(&id) {
            Ok(xs) => match xs { Some(ys) => ys, None => return },
            Err(_) => return
        };
        let commit = match result {
            GitObject::CommitObject(xs) => xs,
            _ => return
        };

        let git_object = match repo.get_path_at_commit(&commit, vec!("src", "stores", "loose.rs")) { 
            Ok(target) => match target {
                Some(git_object) => git_object,
                None => return
            },
            Err(_) => return
        };
        let mut git_blob = match git_object {
            GitObject::BlobObject(xs) => xs,
            _ => return
        };

        let mut contents = String::new();
        let bytes_read = match git_blob.read_to_string(&mut contents) {
            Ok(xs) => xs,
            Err(e) => return
        };
        // println!("blob: {:?}", contents);
    }
}
