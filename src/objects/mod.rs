use crate::pack::internal_type::PackfileType;
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

impl std::convert::Into<PackfileType> for Type {
    fn into(self) -> PackfileType {
        PackfileType::Plain(match self {
            Type::Commit => 1,
            Type::Tree => 2,
            Type::Blob => 3,
            Type::Tag => 4
        })
    }
}

impl std::convert::From<PackfileType> for Type {
    fn from(t: PackfileType) -> Type {
        match t {
            PackfileType::Plain(ident) => {
                match ident {
                    1 => Type::Commit,
                    2 => Type::Tree,
                    3 => Type::Blob,
                    4 => Type::Tag,
                    _ => {
                        panic!("Unknown packfile type")
                    }
                }
            },
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
