pub mod commit;
pub mod blob;
pub mod tree;
pub mod tag;

use std;
use id::Id;
use error::GitError;
use self::commit::Commit;
use self::tree::Tree;
use self::blob::Blob;

trait CanLoad {
    fn from(id: &Id, handle: Box<std::io::Read>) -> Result<&Self, GitError>;
}

pub enum Type {
    Commit(Box<std::io::Read>),
    Tree(Box<std::io::Read>),
    Blob(Box<std::io::Read>),
    Tag(Box<std::io::Read>)
}

impl Type {
    pub fn load<T: CanLoad> (self, i: &Id) -> Result<&T, GitError> {
        match self {
            Type::Commit(t) => T::from(i, t),
            Type::Tree(t) => T::from(i, t),
            Type::Blob(t) => T::from(i, t),
            Type::Tag(t) => T::from(i, t)
        }
    }
}
