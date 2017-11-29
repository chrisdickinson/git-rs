use commit::Commit;
use std::error::Error;

pub enum GitObject {
    CommitObject(Commit),
}
