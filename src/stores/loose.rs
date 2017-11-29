use repository::Repository;
use objects::GitObject;
use stores::Queryable;
use error::GitError;
use id::Id;

#[derive(Debug)]
pub struct Store();

impl Store {
    pub fn new () -> Store {
        Store {
        }
    }
}

impl Queryable for Store {
    fn get (&self, repo: &Repository, id: &Id) -> Result<Option<GitObject>, GitError> {
        Ok(None)
    }
}
