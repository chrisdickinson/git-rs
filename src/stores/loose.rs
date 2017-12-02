use flate2::bufread::DeflateDecoder;
use objects::commit::Commit;
use repository::Repository;
use objects::GitObject;
use stores::Queryable;
use error::GitError;
use std::io::prelude::*;
use std::fs::File;
use std::str;
use std::io;
use id::Id;
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

        let bytes: &[u8; 20] = id.bytes();
        let first = hex::encode(&bytes[0..1]);
        let rest = hex::encode(&bytes[1..20]);
        let mut pb = repo.path().to_path_buf();
        pb.push("objects");
        pb.push(first);
        pb.push(rest);

        let mut file = match File::open(pb.as_path()) {
            Ok(f) => f,
            Err(e) => {
                match e.kind() {
                    io::ErrorKind::NotFound => return Ok(None),
                    _ => return Err(GitError::Unknown)
                }
            }
        };


        let mut bytes = Vec::new();
        let written = match file.read_to_end(&mut bytes) {
            Ok(w) => w,
            Err(e) => {
                return Err(GitError::Unknown)
            }
        };

        let w0 = bytes[0] as u16;
        let w1 = bytes[1] as u16;
        let word = (w0 << 8) + w1;

        // !!! next step is:
        // check to see is_zlib = w0 === 0x78 && !(word % 31)
        // then "commit" | "tree" | "blob" | "tag" SP SIZE NUL body
        let is_deflate = w0 == 0x78 && ((word & 31) != 0);
        let decoded_bytes = if is_deflate {
            let mut decoder = DeflateDecoder::new(&bytes[2..]);
            let mut result = Vec::new();
            match decoder.read_to_end(&mut result) {
                Ok(_) => (),
                Err(e) => {
                    return Err(GitError::Unknown)
                }
            };
            result
        } else {
            bytes.to_vec()
        };

        let type_sp_idx = match decoded_bytes.iter().position(|&xs| xs == 0x20) {
            Some(idx) => idx,
            None => return Err(GitError::Unknown)
        };

        let size_nul_idx = match decoded_bytes.iter().position(|&xs| xs == 0) {
            Some(idx) => idx,
            None => return Err(GitError::Unknown)
        };

        let strtype = match str::from_utf8(&decoded_bytes[0..type_sp_idx]) {
            Ok(xs) => xs,
            Err(e) => return Err(GitError::Unknown)
        };

        match strtype {
            "commit" => Ok(Some(GitObject::CommitObject(Commit::from(id, &decoded_bytes[1 + size_nul_idx..])))),
            &_ => return Err(GitError::Unknown)
        }
    }
}
