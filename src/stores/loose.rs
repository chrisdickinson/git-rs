use flate2::bufread::DeflateDecoder;
use objects::commit::Commit;
use objects::tree::Tree;
use objects::blob::Blob;
use repository::Repository;
use objects::GitObject;
use stores::Queryable;
use error::GitError;
use std::io::prelude::*;
use std::io::BufReader;
use std::fs::File;
use std::str;
use std::io;
use id::Id;
use std;
use hex;

#[derive(Debug)]
pub struct Store();

impl Store {
    pub fn new() -> Store {
        Store {}
    }
}

impl Queryable for Store {
    fn get(&self, repo: &Repository, id: &Id) -> Result<Option<GitObject>, GitError> {

        let bytes: &[u8; 20] = id.as_slice();
        let first = hex::encode(&bytes[0..1]);
        let rest = hex::encode(&bytes[1..20]);
        let mut pb = repo.path().to_path_buf();
        pb.push("objects");
        pb.push(first);
        pb.push(rest);

        let file = match File::open(pb.as_path()) {
            Ok(f) => f,
            Err(e) => {
                match e.kind() {
                    io::ErrorKind::NotFound => return Ok(None),
                    _ => return Err(GitError::Unknown)
                }
            }
        };

        let buffered_file = BufReader::new(file);
        let mut sig_handle = buffered_file.take(2);
        let mut sig_bytes = [0u8; 2];
        match sig_handle.read(&mut sig_bytes) {
            Err(e) => {
                return Err(GitError::Unknown)
            },
            Ok(_) => {}
        };
        let w0 = sig_bytes[0] as u16;
        let w1 = sig_bytes[1] as u16;
        let word = (w0 << 8) + w1;

        let file_after_sig = sig_handle.into_inner();

        // !!! next step is:
        // check to see is_zlib = w0 === 0x78 && !(word % 31)
        // then "commit" | "tree" | "blob" | "tag" SP SIZE NUL body
        let is_deflate = w0 == 0x78 && ((word & 31) != 0);
        let decoder_handle: Box<std::io::Read> = if is_deflate {
            Box::new(DeflateDecoder::new(file_after_sig))
        } else {
            Box::new(file_after_sig)
        };

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
            match next_handle.read(&mut header_byte) {
                Err(e) => return Err(GitError::Unknown),
                Ok(_) => {}
            };
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

        let typename = match str::from_utf8(&type_vec) {
            Ok(xs) => xs,
            Err(e) => return Err(GitError::Unknown)
        };
        let body_handle = header_handle;

        match typename {
            "commit" => Ok(Some(GitObject::CommitObject(Commit::from(id, body_handle)))),
            "tree" => Ok(Some(GitObject::TreeObject(Tree::from(id, body_handle)))),
            "blob" => Ok(Some(GitObject::BlobObject(Blob::from(id, body_handle)))),
            &_ => return Err(GitError::Unknown)
        }
    }
}
