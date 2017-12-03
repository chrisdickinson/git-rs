pub mod commit;
pub mod blob;
pub mod tree;

use self::commit::Commit;
use self::tree::Tree;
use self::blob::Blob;

#[derive(Debug)]
pub enum GitObject {
    CommitObject(Commit),
    TreeObject(Tree),
    BlobObject(Blob),
    TagObject
}
