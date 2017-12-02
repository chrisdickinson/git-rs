pub mod commit;

use self::commit::Commit;

#[derive(Debug)]
pub enum GitObject {
    CommitObject(Commit),
    TreeObject,
    BlobObject,
    TagObject
}
