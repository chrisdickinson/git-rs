use crate::objects::{ CanLoad, Type };
use std::collections::HashMap;
use crate::errors::Result;
use crate::id::Id;

#[derive(Debug)]
pub struct FileMode(u32);

#[derive(Debug)]
pub struct TreeEntry {
    pub mode: FileMode,
    pub id: Id
}

#[derive(Debug)]
pub struct Tree {
    entries: HashMap<Vec<u8>, TreeEntry>
}

impl CanLoad for Tree {
    fn load<T: std::io::Read>(handle: &mut T) -> Result<Type> {
        let mut vec = Vec::new();
        handle.read_to_end(&mut vec);
        let buf = &vec;

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
                    if idx - null < 20 {
                        Mode::CollectHash
                    } else {
                        let name = buf[space + 1..null].to_vec();
                        let mode_str = std::str::from_utf8(&buf[anchor..space])?;
                        let mode = u32::from_str_radix(mode_str, 8)?;

                        entries.insert(name, TreeEntry {
                            mode: FileMode(mode),
                            id: Id::from(&buf[null + 1..idx + 1])
                        });

                        anchor = idx + 1;
                        Mode::FindSpace
                    }
                }
            };
            mode = next
        }

        Ok(Type::Tree(Tree {
            entries: entries
        }))
    }
}

#[cfg(test)]
mod tests {
    use crate::objects::CanLoad;

    #[test]
    fn tree_read_works() {
    }
}
