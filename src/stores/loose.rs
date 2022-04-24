use flate2::bufread::ZlibDecoder;
use std::io::prelude::*;
use std::io::{ BufReader };

use crate::stores::{ Queryable, StorageSet };
use crate::errors::{ Result, ErrorKind };
use crate::objects::Type;
use crate::id::Id;

type Reader = dyn Fn(&Id) -> Result<Option<Box<dyn std::io::Read>>> + Send + Sync;

pub struct Store {
    read: Box<Reader>,
    filter: [bool; 256]
}

impl Store {
    pub fn new<C>(func: C, filter: Option<[bool; 256]>) -> Self
        where C: Fn(&Id) -> Result<Option<Box<dyn std::io::Read>>> + 'static + Send + Sync {
        let filter = match filter {
            Some(xs) => xs,
            None => [true; 256]
        };

        Store {
            read: Box::new(func),
            filter
        }
    }
}

impl Queryable for Store {
    fn get<W: Write, S: Queryable>(&self, id: &Id, output: &mut W, _: &StorageSet<S>) -> Result<Option<Type>> {
        if !self.filter[id.as_ref()[0] as usize] {
            return Ok(None)
        }

        let maybe_reader = (self.read)(id)?;
        if maybe_reader.is_none() {
            return Ok(None)
        }

        let mut reader = BufReader::new(
            ZlibDecoder::new(BufReader::new(maybe_reader.unwrap()))
        );

        let mut type_vec = Vec::new();
        let mut size_vec = Vec::new();

        reader.read_until(0x20, &mut type_vec)?;
        reader.read_until(0, &mut size_vec)?;

        let loaded_type = match &type_vec[..] {
            b"commit " => Type::Commit,
            b"blob " => Type::Blob,
            b"tree " => Type::Tree,
            b"tag " => Type::Tag,
            &_ => return Err(ErrorKind::BadLooseObject.into())
        };

        std::io::copy(&mut reader, output)?;
        Ok(Some(loaded_type))
    }
}

#[cfg(test)]
mod tests {
    use crate::stores::{ Queryable, StorageSet };
    use crate::objects::Object;
    use crate::id::Id;

    use std::io::Cursor;
    use super::{ Store, ErrorKind };

    #[test]
    fn read_commit_works() {
        let store = Store::new(|_| Ok(Some(Box::new(include_bytes!("../../fixtures/loose_commit") as &[u8]))), None);
        let storage_set = StorageSet::new(());

        let mut stream = Vec::new();
        let option = store.get(&Id::default(), &mut stream, &storage_set).expect("it exploded");
        if let Some(xs) = option {
            let mut readable = Cursor::new(stream);
            let object = xs.load(&mut readable).expect("failed to load");

            if let Object::Commit(commit) = object {
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
        let store = Store::new(|_| Ok(Some(Box::new(include_bytes!("../../fixtures/loose_tree") as &[u8]))), None);
        let storage_set = StorageSet::new(());

        let mut stream = Vec::new();
        let option = store.get(&Id::default(), &mut stream, &storage_set).expect("it exploded");
        if let Some(xs) = option {
            let mut readable = Cursor::new(stream);
            let object = xs.load(&mut readable).expect("failed to load");

            if let Object::Tree(tree) = object {
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

    #[test]
    fn handles_idtoreadable_failures() {
        let store = Store::new(|_| Err(ErrorKind::BadLooseObject.into()), None);
        let storage_set = StorageSet::new(());

        match store.get(&Id::default(), &mut vec![], &storage_set) {
            Ok(_) => panic!("expected failure!"),
            Err(e) => assert_eq!(e.description(), "BadLooseObject")
        };
    }

    #[test]
    fn handles_idtoreadable_misses() {
        let store = Store::new(|_| Ok(None), None);
        let storage_set = StorageSet::new(());

        match store.get(&Id::default(), &mut vec![], &storage_set) {
            Err(_) => panic!("expected success!"),
            Ok(xs) => assert!(xs.is_none())
        };
    }
}
