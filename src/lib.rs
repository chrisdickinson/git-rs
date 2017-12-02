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

#[cfg(test)]
mod tests {
    use repository::Repository;
    use std::path::Path;
    use id::Id;
    use objects::GitObject;

    #[test]
    fn it_works() {
        let repo =
            Repository::from_fs(Path::new("/Users/chris/projects/personal/tempisfugit/.git"));

        println!("repo: {:?}", repo);
    }

    #[test]
    fn commits() {
        let repo =
            Repository::from_fs(Path::new("/Users/chris/projects/personal/git-rs/.git"));

        let mut id = match repo.rev_parse("master") {
            Some(xs) => xs,
            None => return
        };
        loop {
            if let Ok(Some(result)) = repo.get_object(&id) {
                if let GitObject::CommitObject(commit) = result {
                    println!("{} {}", id, commit.message().trim());
                    let parents = match commit.parents() {
                        Some(v) => v,
                        None => return
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

        let mut id = match repo.rev_parse("master") {
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
}
