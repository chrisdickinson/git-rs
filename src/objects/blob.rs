use crate::errors::{ Result, ErrorKind };

pub struct Blob {
    contents: Box<std::io::Read>
}

impl Blob {
    pub fn load<T: std::io::Read>(handle: &mut T) -> Result<Blob> {
        Err(ErrorKind::NotImplemented.into())
    }
}
