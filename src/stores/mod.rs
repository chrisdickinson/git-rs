pub mod loose;
pub mod pack;

use id::Id;
use objects::GitObject;
use error::GitError;
use std::fmt::Debug;

pub trait Queryable : Debug {
    fn get (&self, id: &Id) -> Result<Option<GitObject>, GitError>;
}
