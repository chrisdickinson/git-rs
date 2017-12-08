use std::fmt::Debug;

pub mod loose;
pub mod index;
pub mod pack;

use id::Id;
use error::GitError;
use objects::GitObject;
use repository::Repository;

pub trait Queryable: Debug {
    fn get(&self, repo: &Repository, id: &Id) -> Result<Option<GitObject>, GitError>;
}
