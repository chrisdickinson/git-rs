use flate2::bufread::DeflateDecoder;
use std::io::prelude::*;
use std::io::{ BufReader, Read, BufRead };
use std::path::Path;
use std::fs::File;

use crate::errors::{ Result, ErrorKind };
use crate::objects::CanLoad;
use crate::objects::commit::Commit;
use crate::objects::blob::Blob;
use crate::objects::tree::Tree;
use crate::objects::tag::Tag;
use crate::objects::Type;
use crate::id::Id;

pub trait IdToReadable {
    type Reader;

    fn read(self: &Self, id: &Id) -> Result<Option<Self::Reader>>;
}

#[derive(Debug)]
pub struct Store<T: IdToReadable> {
    reader: T
}

struct LooseFS {
    root: Path
}

impl IdToReadable for LooseFS {
    type Reader = std::fs::File;

    fn read(&self, id: &Id) -> Result<Option<Self::Reader>> {
        let as_str = id.to_string();
        let mut pb = self.root.to_path_buf();
        pb.push(as_str[0..2].to_string());
        pb.push(as_str[2..40].to_string());
        match File::open(pb.as_path()) {
            Ok(f) => Ok(Some(f)),
            Err(e) => {
                match e.kind() {
                    std::io::ErrorKind::NotFound => return Ok(None),
                    _ => return Err(e)?
                }
            }
        }
    }
}

impl<T: IdToReadable> Store<T> where T::Reader : Read {
    fn get(&self, id: &Id) -> Result<Option<Type>> {
        let maybe_reader = self.reader.read(id)?;
        if maybe_reader.is_none() {
            return Ok(None)
        }

        let reader = BufReader::new(maybe_reader.unwrap());
        let mut sig_handle = reader.take(2);
        let mut sig_bytes = [0u8; 2];
        match sig_handle.read(&mut sig_bytes) {
            Err(e) => {
                return Err(ErrorKind::BadLooseObject.into())
            },
            Ok(_) => {}
        };
        let w0 = sig_bytes[0] as u16;
        let w1 = sig_bytes[1] as u16;
        let word = (w0 << 8) + w1;

        let mut file_after_sig = sig_handle.into_inner();

        // !!! next step is:
        // check to see is_zlib = w0 === 0x78 && !(word % 31)
        // then "commit" | "tree" | "blob" | "tag" SP SIZE NUL body
        let is_deflate = w0 == 0x78 && ((word & 31) != 0);
        return if is_deflate {
            self.inner_read(&mut DeflateDecoder::new(file_after_sig))
        } else {
            self.inner_read(&mut file_after_sig)
        }
    }

    fn inner_read<S: Read>(&self, decoder_handle: &mut S) -> Result<Option<Type>> {
        let mut type_vec = Vec::new();
        let mut size_vec = Vec::new();
        enum Mode {
            FindSpace,
            FindNull
        };
        let mut mode = Mode::FindSpace;

        let mut header_handle = decoder_handle;
        loop {
            let mut next_handle = header_handle.take(1);
            let mut header_byte = [0u8; 1];
            next_handle.read(&mut header_byte)?;
            let next = match mode {
                Mode::FindSpace => {
                    match header_byte[0] {
                        0x20 => {
                            Mode::FindNull
                        },
                        xs => {
                            type_vec.push(xs);
                            Mode::FindSpace
                        }
                    }
                },
                Mode::FindNull => {
                    match header_byte[0] {
                        0x0 => {
                            header_handle = next_handle.into_inner();
                            break
                        },
                        xs => {
                            size_vec.push(xs);
                            Mode::FindNull
                        }
                    }
                }
            };
            mode = next;
            header_handle = next_handle.into_inner();
        }

        let typename = std::str::from_utf8(&type_vec)?;
        let body_handle = header_handle;

        let loaded_type = match typename {
            "commit" => Commit::load(body_handle),
            // "blob" => Blob::load(body_handle),
            "tree" => Tree::load(body_handle),
            "tag" => Tag::load(body_handle),
            &_ => return Err(ErrorKind::BadLooseObject.into())
        }?;

        Ok(Some(loaded_type))
    }
}

#[cfg(test)]
mod tests {
    use crate::objects::CanLoad;
    use crate::objects::Type;
    use crate::id::Id;
    use crate::objects::tree::FileMode;

    use super::{ IdToReadable, Store, Result, ErrorKind };

    struct FakeFS<'a> {
        bytes: &'a [u8]
    }

    impl<'a> IdToReadable for FakeFS<'a> {
        type Reader = &'a [u8];

        fn read(&self, _: &Id) -> Result<Option<Self::Reader>> {
            Ok(Some(self.bytes))
        }
    }

    #[test]
    fn read_commit_works() {
        let store = Store {
            reader: FakeFS {
                bytes: include_bytes!("../../fixtures/loose_commit")
            }
        };

        let option = store.get(&Id::default()).expect("it exploded");
        if let Some(xs) = option {
            if let Type::Commit(commit) = xs {
                let message = std::str::from_utf8(commit.message()).expect("not utf8");
                assert_eq!(message, "maybe implement loose store\n");
            } else {
                panic!("expected commit");
            }
        } else {
            panic!("explode");
        }
    }

    #[test]
    fn read_tree_works() {
        let store = Store {
            reader: FakeFS {
                bytes: include_bytes!("../../fixtures/loose_tree")
            }
        };

        let option = store.get(&Id::default()).expect("it exploded");
        if let Some(xs) = option {
            if let Type::Tree(tree) = xs {
                let mut entries: Vec<&str> = tree.entries().keys()
                    .map(|xs| ::std::str::from_utf8(xs).expect("valid utf8"))
                    .collect();
                entries.sort();
                assert_eq!(entries.join("\n"), ".gitignore\nCargo.toml\nREADME.md\nfixtures\nsrc");
            } else {
                panic!("expected tree");
            }
        } else {
            panic!("explode");
        }
    }

    struct FailFS;

    impl IdToReadable for FailFS {
        type Reader = std::fs::File;

        fn read(&self, _: &Id) -> Result<Option<Self::Reader>> {
            Err(ErrorKind::BadLooseObject.into())
        }
    }

    #[test]
    fn handles_idtoreadable_failures() {
        let store = Store {
            reader: FailFS { }
        };

        match store.get(&Id::default()) {
            Ok(_) => panic!("expected failure!"),
            Err(e) => assert_eq!(e.description(), "BadLooseObject")
        };
    }

    struct MissingFS;

    impl IdToReadable for MissingFS {
        type Reader = std::fs::File;

        fn read(&self, _: &Id) -> Result<Option<Self::Reader>> {
            Ok(None)
        }
    }

    #[test]
    fn handles_idtoreadable_misses() {
        let store = Store {
            reader: MissingFS { }
        };

        match store.get(&Id::default()) {
            Err(_) => panic!("expected success!"),
            Ok(xs) => assert!(xs.is_none())
        };
    }
}
