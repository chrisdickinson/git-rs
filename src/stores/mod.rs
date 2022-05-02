use std::fmt::Debug;

pub mod loose;
pub mod pack;

use id::Id;
use objects::Type;
use error::GitError;
use repository::Repository;

pub trait Queryable: Debug {
    fn get(&self, repo: &Repository, id: &Id) -> Result<Option<Type>, GitError>;
}
