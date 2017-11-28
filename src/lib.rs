extern crate hex;
extern crate glob;

mod id;
mod reference;
mod repository;

use std::path::{Path, PathBuf};
use repository::Repository;

#[cfg(test)]
mod tests {
    use ::Repository;
    use ::Path;

    #[test]
    fn it_works() {
        let repo = Repository::new(
            Path::new("/Users/chris/projects/personal/tempisfugit/.git")
        );

        println!("repo: {:?}", repo);
    }
}
