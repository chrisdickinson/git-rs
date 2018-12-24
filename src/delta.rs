use std;

pub const OFS_DELTA: u8 = 6;
pub const REF_DELTA: u8 = 7;


// deltas read two varints:
// - 1: the base size info
pub struct DeltaDecoder<T: std::io::Read> {
    instructions: Vec<u8>,
    index: usize,
    inner: T
}

impl<T: std::io::Read> DeltaDecoder<T> {
    pub fn new (instructions: &[u8], base: T) -> DeltaDecoder<T> {
        DeltaDecoder {
            instructions: Vec::from(instructions),
            index: 0,
            inner: base
        }
    }
}

// read the size out of the 
impl<T: std::io::Read> std::io::Read for DeltaDecoder<T> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}

