use id::Id;
use std::vec::Vec;
use std::error::Error;
use std::collections::HashMap;

use repository::Repository;

#[derive(Debug)]
pub struct Commit {
    id: Id,
    attributes: HashMap<String, Vec<String>>,
    message: String
}

impl Commit {
    pub fn from (id: &Id, buf: &[u8]) {
        // layout is:
        // attr SP value NL
        // NL
        // message
    }

    pub fn authors (&self) -> Option<&Vec<String>> {
        self.attributes.get("author")
    }

    pub fn message (&self) -> &str {
        self.message.as_str()
    }

    pub fn parents (&self, repo: &Repository) {

    }

    // pub fn tree (&self) -> &Tree {
    // }
}
