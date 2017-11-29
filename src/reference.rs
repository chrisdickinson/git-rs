use id::Id;
use std::fs::File;
use std::io::Result;
use std::io::prelude::*;

#[derive(Debug)]
pub struct Ref {
    id: Id,
}

impl Ref {
    pub fn new(path: &str) -> Result<Self> {
        let mut file = File::open(path)?;
        let mut contents = String::new();

        let written = file.read_to_string(&mut contents)?;
        let id = Id::from(&contents);

        Ok(Ref { id: id })
    }
}
