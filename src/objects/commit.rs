use std::collections::HashMap;

use crate::human_metadata::HumanMetadata;
use crate::errors::Result;
use crate::id::Id;

#[derive(Debug)]
pub struct Commit {
    attributes: HashMap<Vec<u8>, Vec<Vec<u8>>>,
    committer: Option<HumanMetadata>,
    author: Option<HumanMetadata>,
    message: Vec<u8>
}

impl Commit {
    pub fn message(&self) -> &[u8] {
        self.message.as_slice()
    }

    pub fn committer(&self) -> &Option<HumanMetadata> {
        &self.committer
    }

    pub fn author(&self) -> &Option<HumanMetadata> {
        &self.author
    }

    pub fn tree(&self) -> Option<Id> {
        let v = self.attributes.get(b"tree" as &[u8])?;

        let mut result: Vec<Id> = v.iter().filter_map(|id_bytes| {
            std::str::from_utf8(&id_bytes).ok().and_then(|xs| xs.parse().ok())
        }).collect();
        result.pop()
    }

    pub fn parents(&self) -> Option<Vec<Id>> {
        let v = self.attributes.get(b"parent" as &[u8])?;
        let result: Vec<Id> = v.iter().filter_map(|id_bytes| {
            std::str::from_utf8(&id_bytes).ok().and_then(|xs| xs.parse().ok())
        }).collect();
        Some(result)
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
        for (idx, byte) in buf.iter().enumerate() {
            let next = match mode {
                Mode::Attr => {
                    match *byte {
                        0x20 => {
                            space = idx;
                            Mode::Value
                        },
                        0x0a => {
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
                        0x0a => {
                            let key = buf[anchor..space].to_vec();
                            let value = buf[space + 1..idx].to_vec();
                            attributes
                                .entry(key)
                                .or_insert_with(Vec::new)
                                .push(value);
                            anchor = idx + 1;
                            space = idx;
                            Mode::Attr
                        },
                        _ => Mode::Value
                    }
                }
            };

            mode = next;
        }

        let message = buf[message_idx..].to_vec();

        let committer = attributes.get_mut(b"committer" as &[u8]).and_then(|xs| {
            xs.pop()
        }).map(HumanMetadata::new);

        let author = attributes.get_mut(b"author" as &[u8]).and_then(|xs| {
            xs.pop()
        }).map(HumanMetadata::new);

        Ok(Commit {
            attributes,
            committer,
            message,
            author
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
