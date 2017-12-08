use std::error;
use std::fmt;
use std;
use hex;

#[derive(Debug)]
pub enum GitError {
    Unknown,
    InvalidID(hex::FromHexError),
    BadReference(std::io::Error),
    BadPackfileIndex(std::io::Error)
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            GitError::Unknown => f.write_str("Unknown git error"),
            _ => f.write_str("Other error")
        }
    }
}

impl error::Error for GitError {
    fn description(&self) -> &str {
        match *self {
            GitError::Unknown => "Unknown git error",
            _ => "Other error"
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        None
    }
}
