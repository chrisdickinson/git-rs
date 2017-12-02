use error::GitError;
use std::fmt;
use hex;

#[derive(Clone, PartialEq, Eq)]
pub struct Id {
    id: [u8; 20],
}

impl fmt::Debug for Id {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        hex::encode(self.id).fmt(formatter)
    }
}

impl fmt::Display for Id {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        hex::encode(self.id).fmt(formatter)
    }
}

impl Id {
    pub fn from(inp: &str) -> Result<Id, GitError> {
        let mut identifier = Id { id: [0u8; 20] };
        let bytes = match hex::decode(inp.trim()) {
            Ok(xs) => xs,
            Err(e) => return Err(GitError::InvalidID(e)),
        };
        identifier.id.clone_from_slice(&bytes);
        Ok(identifier)
    }

    pub fn from_bytes(inp: &[u8]) -> Id {
        let mut dst = [0u8; 20];
        dst.clone_from_slice(&inp);
        Id {
            id: dst
        }
    }

    pub fn bytes (&self) -> &[u8; 20] {
        &self.id
    }
}
