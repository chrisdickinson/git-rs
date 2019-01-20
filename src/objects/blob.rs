use crate::errors::Result;

pub struct Blob {
    pub contents: Vec<u8>
}

impl Blob {
    pub fn load<T: std::io::Read>(handle: &mut T) -> Result<Blob> {
        let mut contents = Vec::new();
        handle.read_to_end(&mut contents)?;

        Ok(Blob {
            contents
        })
    }
}
