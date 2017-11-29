use std::error;
use std::fmt;

#[derive(Debug)]
pub enum GitError {
    Unknown,
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Unknown => f.write_str("Unknown git error"),
        }
    }
}

impl error::Error for GitError {
    fn description(&self) -> &str {
        match self {
            Unknown => "Unknown Git error",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        None
    }
}
