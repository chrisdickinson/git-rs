pub mod commit;
pub mod tree;

use self::commit::Commit;
use self::tree::Tree;

#[derive(Debug)]
pub enum GitObject {
    CommitObject(Commit),
    TreeObject(Tree),
    BlobObject,
    TagObject
}
