use std::collections::HashMap;
use crate::errors::Result;
use crate::objects::Type;
use crate::id::Id;

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
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

impl Tree {
    pub fn entries (&self) -> &HashMap<Vec<u8>, TreeEntry> {
        &self.entries
    }
}

impl IntoIterator for Tree {
    type Item = (Vec<u8>, TreeEntry);
    type IntoIter = std::collections::hash_map::IntoIter<Vec<u8>, TreeEntry>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}

impl Tree {
    pub fn load<T: std::io::Read>(handle: &mut T) -> Result<Tree> {
        let mut vec = Vec::new();
        handle.read_to_end(&mut vec)?;
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

        Ok(Tree {
            entries: entries
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::id::Id;
    use crate::objects::tree::FileMode;

    #[test]
    fn tree_read_works() {
        let bytes = include_bytes!("../../fixtures/tree");
        let tree = super::Tree::load(&mut bytes.as_ref()).expect("oh no");
        let tree_entry = tree.entries.get("src".as_bytes()).unwrap();
        assert_eq!(tree_entry.id, Id::from_str("dc222eddb8d03b5f7cac5e7909dd400f3ce33935").unwrap());
        assert_eq!(tree_entry.mode, FileMode(0o40000));
    }

    #[test]
    fn tree_complex_read_works() {
        let bytes = include_bytes!("../../fixtures/tree_1");
        let tree = super::Tree::load(&mut bytes.as_ref()).expect("oh no");
        let mut entries: Vec<&str> = tree.entries.keys()
            .map(|xs| ::std::str::from_utf8(xs).expect("valid utf8"))
            .collect();
        entries.sort();

        assert_eq!(entries.join("\n"), "errors.rs\nid.rs\nlib.rs\nobjects");
        let tree_entries = entries.iter().map(|xs| tree.entries.get(xs.as_bytes()).unwrap());
        let modes: Vec<FileMode> = tree_entries.clone().map(|xs| xs.mode.clone()).collect();
        assert_eq!(
            modes,
            vec![FileMode(0o100644), FileMode(0o100644), FileMode(0o100644), FileMode(0o40000)]
        );
        let ids: Vec<Id> = tree_entries.map(|xs| xs.id.clone()).collect();
        assert_eq!(
            ids,
            vec![
                Id::from_str("b83365ae6f3559a72061c9c8e0ff55c781017c00").unwrap(),
                Id::from_str("fb96e2a4b5142b7acc136a3a653af831c7a3fcf3").unwrap(),
                Id::from_str("b31c74004136914690c436cde6a3c60023ea825c").unwrap(),
                Id::from_str("fbd42d37eddf656834a9a3e470ddb0dfa8a65a32").unwrap(),
            ]
        );
    }
}
