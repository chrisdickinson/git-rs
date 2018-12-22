use crate::errors::Result;
use crate::id::Id;

pub mod commit;
pub mod blob;
pub mod tree;
pub mod tag;

trait CanLoad {
    fn load<T: std::io::Read>(handle: &mut T) -> Result<Type>;
}

pub enum Type {
    Commit(crate::objects::commit::Commit),
    Tree(crate::objects::tree::Tree),
    Blob(self::blob::Blob),
    Tag(self::tag::Tag)
}
