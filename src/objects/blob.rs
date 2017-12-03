use std;
use id::Id;
use std::cell::RefCell;

enum BlobContents {
    Pending(Box<std::io::Read>),
    Resident(Vec<u8>)
}

#[derive(Debug)]
pub struct Blob {
    id: Id,
    data: RefCell<BlobContents>
}

impl std::fmt::Debug for BlobContents {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            BlobContents::Pending(_) => formatter.write_str(
                "BlobContents { <pending> }"),
            BlobContents::Resident(ref vec) => formatter.write_str(
                "BlobContents { <resident> }")
        }
    }
}


impl Blob {
    pub fn from (id: &Id, mut handle: Box<std::io::Read>) -> Blob {
        let mut reader = handle;

        Blob {
            id: Id::clone(id),
            data: RefCell::new(BlobContents::Pending(reader))
        }
    }
}
