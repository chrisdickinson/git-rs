use std::fmt;
use hex;

#[derive(PartialEq, Eq)]
pub struct Id {
    id: [u8; 20]
}

impl fmt::Debug for Id {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        hex::encode(self.id).fmt(formatter)
    }
}

impl Id {
    pub fn from (inp: &str) -> Id {
        let mut identifier = Id {
            id: [0u8; 20]
        };
        let bytes = match hex::decode(inp.trim()) {
            Ok(xs) => xs,
            Err(e) => return identifier
        };
        identifier.id.clone_from_slice(&bytes);
        identifier
    }
}
