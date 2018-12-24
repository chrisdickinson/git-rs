use crate::errors::Result;
use crate::id::Id;

pub mod commit;
pub mod blob;
pub mod tree;
pub mod tag;

pub trait CanLoad {
    fn load<T: std::io::Read>(handle: &mut T) -> Result<Type>;
}

pub enum Type {
    Commit(Box<std::io::Read>),
    Tree(Box<std::io::Read>),
    Blob(Box<std::io::Read>),
    Tag(Box<std::io::Read>)
}
