extern crate hex;
extern crate glob;

mod repository;
mod reference;
mod objects;
mod commit;
mod stores;
mod error;
mod id;

use std::path::{Path, PathBuf};
use repository::Repository;

#[cfg(test)]
mod tests {
    use ::Repository;
    use ::Path;

    #[test]
    fn it_works() {
        let repo = Repository::from_fs(
            Path::new("/Users/chris/projects/personal/tempisfugit/.git")
        );

        println!("repo: {:?}", repo);
    }
}
