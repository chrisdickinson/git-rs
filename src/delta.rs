use std;
use stores::pack::Store;
use repository::Repository;

pub const OFS_DELTA: u8 = 6;
pub const REF_DELTA: u8 = 7;

pub struct DeltaDecoder {
    instructions: Vec<u8>,
    base: Box<std::io::Read>
}

// - stores need to expose "raw read" (give me a Box'd stream)
// - pack store needs to expose "raw read at offset"
//

impl DeltaDecoder {
    pub fn new (instructions: &[u8], base: Box<std::io::Read>) -> DeltaDecoder {
        DeltaDecoder {
            instructions: Vec::from(instructions),
            base: base
        }
    }
}

impl std::io::Read for DeltaDecoder {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.base.read(buf)
    }
}

