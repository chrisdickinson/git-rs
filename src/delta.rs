use std;
use stores::pack::Store;
use repository::Repository;

pub const OFS_DELTA: u8 = 6;
pub const REF_DELTA: u8 = 7;

#[derive(Debug)]
pub struct DeltaDecoder {
//    inner: Box<std::io::Read>,
//    base: Box<std::io::Read>
}

// - stores need to expose "raw read" (give me a Box'd stream)
// - pack store needs to expose "raw read at offset"
//

impl DeltaDecoder {
    pub fn from_offset_delta (stream: Box<std::io::Read>, store: &Store) -> DeltaDecoder {
        DeltaDecoder {}
    }

    pub fn from_ref_delta (stream: Box<std::io::Read>, repo: &Repository) -> DeltaDecoder {
        DeltaDecoder {}
    }

    pub fn get_type(&self) -> u8 {
        0xff
    }
}

impl std::io::Read for DeltaDecoder {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        Ok(0)
    }
}

