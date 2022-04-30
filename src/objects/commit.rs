use std::collections::HashMap;

use crate::human_metadata::HumanMetadata;
use crate::errors::Result;
use crate::id::Id;

#[derive(Debug)]
pub struct Commit {
    attributes: HashMap<Vec<u8>, Vec<Vec<u8>>>,
    committer: Option<HumanMetadata>,
    authors: Vec<HumanMetadata>,
    parents: Vec<Id>,
    tree: Option<Id>,
    message: Vec<u8>
}

impl Commit {
    pub fn message(&self) -> &[u8] {
        self.message.as_slice()
    }

    pub fn committer(&self) -> &Option<HumanMetadata> {
        &self.committer
    }

    pub fn author(&self) -> Option<&HumanMetadata> {
        self.authors.first()
    }

    pub fn tree(&self) -> Option<&Id> {
        self.tree.as_ref()
    }

    pub fn parents(&self) -> &[Id] {
        self.parents.as_slice()
    }
}

impl AsRef<HashMap<Vec<u8>, Vec<Vec<u8>>>> for Commit {
    fn as_ref(&self) -> &HashMap<Vec<u8>, Vec<Vec<u8>>> {
        &self.attributes
    }
}

impl Commit {
    pub fn load<T: std::io::Read>(handle: &mut T) -> Result<Commit> {
        // attr SP value NL
        // NL
        // message
        let mut vec = Vec::with_capacity(512);
        handle.read_to_end(&mut vec)?;
        let buf = &vec;

        #[derive(Debug)]
        enum Mode {
            Attr,
            Value
        }

        let mut anchor = 0;
        let mut space = 0;
        let mut mode = Mode::Attr;
        let mut message_idx = 0;

        let mut attributes = HashMap::new();

        let mut authors = Vec::new();
        let mut committer = None;
        let mut parents = Vec::new();
        let mut tree = None;

        for (idx, ref byte) in buf.iter().enumerate() {
            mode = match mode {
                Mode::Attr => {
                    match byte {
                        b' ' => {
                            space = idx;
                            Mode::Value
                        },
                        b'\n' => {
                            if anchor == idx {
                                message_idx = idx + 1;
                                break
                            }
                            Mode::Attr
                        },
                        _ => Mode::Attr
                    }
                },

                Mode::Value => {
                    match *byte {
                        b'\n' => {
                            match &buf[anchor..space] {
                                b"author" => {
                                    authors.push(HumanMetadata::new(buf[space + 1..idx].to_vec()));
                                },

                                b"parent" => {
                                    parents.push(Id::new_from_ascii_bytes(&buf[space + 1..idx])?)
                                },

                                b"committer" => {
                                    committer.replace(HumanMetadata::new(buf[space + 1..idx].to_vec()));
                                },

                                b"tree" => {
                                    tree.replace(Id::new_from_ascii_bytes(&buf[space + 1..idx])?);
                                },

                                // TODO: gpgsig is unlike other attributes: it may span multiple lines.
                                // It'll need to be handled specifically.
                                b"gpgsig" => {},

                                _ => {
                                    let key = buf[anchor..space].to_vec();
                                    let value = buf[space + 1..idx].to_vec();
                                    attributes
                                        .entry(key)
                                        .or_insert_with(Vec::new)
                                        .push(value);
                                }
                            }

                            anchor = idx + 1;
                            space = idx;
                            Mode::Attr
                        },
                        _ => Mode::Value
                    }
                }
            };
        }

        let message = buf[message_idx..].to_vec();

        Ok(Commit {
            attributes,
            committer,
            message,
            parents,
            tree,
            authors
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn commit_read_works() {
        let bytes = include_bytes!("../../fixtures/commit");
        let commit = super::Commit::load(&mut bytes.as_ref()).expect("oh no");
        let message = std::str::from_utf8(&commit.message).expect("not utf8");
        assert_eq!(message, "initial commit\n\n");
    }
}
