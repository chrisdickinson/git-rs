use std::collections::HashMap;
use std::str;
use id::Id;

#[derive(Debug)]
pub struct FileMode(u32);

#[derive(Debug)]
pub struct TreeEntry {
    mode: FileMode,
    id: Id
}

#[derive(Debug)]
pub struct Tree {
    id: Id,
    entries: HashMap<String, TreeEntry>
}

// layout is: ascii octal mode SP name NUL hex*20
impl Tree {
    pub fn from(id: &Id, buf: &[u8]) -> Tree {

        #[derive(Debug)] 
        enum Mode {
            FindSpace,
            FindNull,
            CollectHash
        }
        let mut entries = HashMap::new();
        let mut anchor = 0;
        let mut space = 0;
        let mut null = 0;
        let mut mode = Mode::FindSpace;

        for (idx, byte) in buf.iter().enumerate() {
            let next = match mode {
                Mode::FindSpace => {
                    if *byte == 0x20 {
                        space = idx;
                        Mode::FindNull
                    } else {
                        Mode::FindSpace
                    }
                },
                Mode::FindNull => {
                    if *byte == 0 {
                        null = idx;
                        Mode::CollectHash
                    } else {
                        Mode::FindNull
                    }
                },
                Mode::CollectHash => {
                    if idx - null < 21{
                        Mode::CollectHash
                    } else {
                        let mode_str = match str::from_utf8(&buf[anchor..space]) {
                            Ok(xs) => xs,
                            Err(e) => break
                        };
                        let mode = match u32::from_str_radix(mode_str, 8) {
                            Ok(xs) => xs,
                            Err(e) => break
                        };
                        let name = match str::from_utf8(&buf[space + 1..null]) {
                            Ok(xs) => xs,
                            Err(e) => break
                        };

                        entries.insert(name.to_string(), TreeEntry {
                            mode: FileMode(mode),
                            id: Id::from_bytes(&buf[null + 1..idx])
                        });

                        anchor = idx + 1;
                        Mode::FindSpace
                    }
                }
            };
            mode = next
        }

        Tree {
            entries: entries,
            id: Id::clone(id)
        }
    }
}

