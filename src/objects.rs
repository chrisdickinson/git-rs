use commit::Commit;
use std::error::Error;

#[derive(Debug)]
pub enum GitObject {
    CommitObject(Commit),
    TreeObject,
    BlobObject,
    TagObject
}
