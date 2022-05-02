use std;
use id::Id;
use std::str;
use std::vec::Vec;
use multimap::MultiMap;

#[derive(Debug)]
pub struct Tag {
    id: Id,
    attributes: MultiMap<String, String>,
    message: String,
}

impl Tag {
    pub fn from(id: &Id, mut handle: Box<std::io::Read>) -> Tag {
        // layout is:
        // attr SP value NL
        // NL
        // message

        let mut vec = Vec::new();
        let r2e_result = handle.read_to_end(&mut vec);
        let buf = &vec;

        #[derive(Debug)]
        enum Mode {
            Attr,
            Value
        };
        let mut anchor = 0;
        let mut space = 0;
        let mut mode = Mode::Attr;
        let mut message_idx = 0;

        let mut attributes = MultiMap::new();
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
                            let key = match str::from_utf8(&buf[anchor..space]) {
                                Ok(xs) => xs,
                                Err(e) => break
                            };
                            let value = match str::from_utf8(&buf[space + 1..idx]) {
                                Ok(xs) => xs,
                                Err(e) => break
                            };
                            attributes.insert(key.to_string(), value.to_string());
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

        let message = match str::from_utf8(&buf[message_idx..]) {
            Ok(xs) => xs,
            Err(e) => "<Bad UTF8>"
        };
        Tag {
            id: Id::clone(id),
            attributes: attributes,
            message: message.to_string()
        }
    }

    pub fn authors(&self) -> Option<&Vec<String>> {
        self.attributes.get_vec("author")
    }

    pub fn message(&self) -> &str {
        self.message.as_str()
    }

    pub fn tree(&self) -> Option<&String> {
        self.attributes.get("tree")
    }

    pub fn parents(&self) -> Option<&Vec<String>> {
        self.attributes.get_vec("parent")
    }

    // pub fn tree (&self) -> &Tree {
    // }
}

