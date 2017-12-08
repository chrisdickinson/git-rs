use std;
use id::Id;
use std::cell::RefCell;

pub struct Blob {
    id: Id,
    contents: Box<std::io::Read>
}

impl std::fmt::Debug for Blob {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("Blob { }")
    }
}

impl Blob {
    pub fn from (id: &Id, mut handle: Box<std::io::Read>) -> Blob {
        let mut reader = handle;

        Blob {
            id: Id::clone(id),
            contents: reader
        }
    }
}

impl std::io::Read for Blob {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.contents.read(buf)
    }
}
