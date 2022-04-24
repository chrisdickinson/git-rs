use crate::pack::internal_type::PackfileType;
use std::convert::{ TryFrom, From };
use crate::errors::Result;

pub mod commit;
pub mod blob;
pub mod tree;
pub mod tag;

#[derive(Copy, Clone, Debug)]
pub enum Type {
    Commit,
    Tree,
    Blob,
    Tag
}

pub enum Object {
    Commit(commit::Commit),
    Tree(tree::Tree),
    Blob(blob::Blob),
    Tag(tag::Tag)
}

impl From<Type> for PackfileType {
    fn from(t: Type) -> PackfileType {
        PackfileType::Plain(t)
    }
}

use crate::errors::{ ErrorKind, Error };

impl TryFrom<u8> for Type {
    type Error = Error;

    fn try_from(t: u8) -> Result<Type> {
        match t {
            1 => Ok(Type::Commit),
            2 => Ok(Type::Tree),
            3 => Ok(Type::Blob),
            4 => Ok(Type::Tag),

            _ => Err(ErrorKind::InvalidObjectType.into())
        }
    }
}


impl TryFrom<PackfileType> for Type {
    type Error = Error;

    fn try_from(t: PackfileType) -> Result<Type> {
        match t {
            PackfileType::Plain(ident) => Ok(ident),

            _ => {
                panic!("Cannot convert delta packfile type to external type")
            }
        }
    }
}

impl Type {
    pub fn as_str(&self) -> &'static str {
        match self {
            Type::Commit => "commit",
            Type::Tree => "tree",
            Type::Blob => "blob",
            Type::Tag => "tag"
        }
    }

    pub fn load<T: std::io::Read>(&self, stream: &mut T) -> Result<Object> {
        match &self {
            Type::Commit => {
                let xs = commit::Commit::load(stream)?;
                Ok(Object::Commit(xs))
            },
            Type::Tree => {
                let xs = tree::Tree::load(stream)?;
                Ok(Object::Tree(xs))
            },
            Type::Tag => {
                let xs = tag::Tag::load(stream)?;
                Ok(Object::Tag(xs))
            },
            Type::Blob => {
                let xs = blob::Blob::load(stream)?;
                Ok(Object::Blob(xs))
            }
        }
    }
}
