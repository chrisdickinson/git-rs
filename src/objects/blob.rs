use std;
use id::Id;
use objects::CanLoad;
use error::GitError;

pub struct Blob {
    id: Id,
    contents: Box<std::io::Read>
}

impl std::fmt::Debug for Blob {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("Blob { }")
    }
}

impl CanLoad for Blob {
    pub fn from (id: &Id, mut handle: Box<std::io::Read>) -> Result<&Self, GitError> {
        let mut reader = handle;

        Ok(&Blob {
            id: Id::clone(id),
            contents: reader
        })
    }
}

impl std::io::Read for Blob {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.contents.read(buf)
    }
}
